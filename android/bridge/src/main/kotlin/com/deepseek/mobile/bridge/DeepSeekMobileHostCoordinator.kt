package com.deepseek.mobile.bridge

import android.app.PendingIntent
import android.content.Intent
import androidx.activity.ComponentActivity
import androidx.core.content.FileProvider
import java.io.File

/**
 * Drains Rust native-bridge commands and dispatches them to the Kotlin bridge adapters.
 *
 * The Dioxus/Android shell must implement [NativeBridgeBindings] (JNI or future interop)
 * and call [pollAndHandleNextAction] on each UI tick while the app is foregrounded.
 */
class DeepSeekMobileHostCoordinator(
    private val activity: ComponentActivity,
    private val bindings: NativeBridgeBindings,
    private val documentPickerBridge: DeepSeekDocumentPickerBridge,
    private val discoveryBridge: DeepSeekPcGatewayDiscoveryBridge,
    private val termuxBridge: DeepSeekTermuxBridge,
    private val termuxPendingIntentFactory: (String) -> PendingIntent,
) {
    fun pollAndHandleNextAction(): Boolean {
        val actionJson = bindings.pollNextHostActionJson() ?: return false
        handleActionJson(actionJson)
        return true
    }

    fun deliverCallbackJson(payload: String) {
        bindings.deliverHostCallbackJson(payload)
    }

    fun handleActionJson(actionJson: String) {
        val action = AndroidHostActionJson.parse(actionJson) ?: return
        when (action.kind) {
            "open_document_picker" -> {
                documentPickerBridge.open(
                    AndroidDocumentPickerCommandPayload(
                        requestId = action.requestId ?: return,
                        action = action.androidAction ?: Intent.ACTION_OPEN_DOCUMENT,
                        category = action.androidCategory ?: Intent.CATEGORY_OPENABLE,
                        allowMultiple = action.allowMultiple ?: false,
                        mimeTypes = action.mimeTypes ?: listOf("*/*"),
                    )
                )
            }
            "start_pc_gateway_discovery" -> {
                discoveryBridge.startDiscovery(
                    AndroidPcGatewayDiscoveryCommandPayload(
                        requestId = action.requestId ?: return,
                        serviceType = action.serviceType ?: "_deepseek-pc-gateway._tcp.",
                    )
                )
            }
            "run_termux_command" -> {
                val requestId = action.requestId ?: return
                val command = action.command ?: return
                val workingDir = action.workingDir ?: return
                val pending = termuxPendingIntentFactory(requestId)
                termuxBridge.run(
                    AndroidTermuxCommandPayload(
                        requestId = requestId,
                        commandPath = "/data/data/com.termux/files/usr/bin/sh",
                        arguments = listOf("-lc", command),
                        workingDir = workingDir,
                        background = true,
                        label = "DeepSeek Mobile",
                        description = command,
                    ),
                    pending,
                )
            }
            "share_file" -> {
                val path = action.path ?: return
                val file = File(path)
                if (!file.exists()) {
                    deliverCallbackJson(
                        """{"kind":"share_failed","message":"file not found: $path"}"""
                    )
                    return
                }
                val uri = FileProvider.getUriForFile(
                    activity,
                    "${activity.packageName}.fileprovider",
                    file,
                )
                val intent = Intent(Intent.ACTION_SEND).apply {
                    type = action.mimeType ?: "application/octet-stream"
                    putExtra(Intent.EXTRA_STREAM, uri)
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                activity.startActivity(Intent.createChooser(intent, "Share project"))
                deliverCallbackJson("""{"kind":"file_shared"}""")
            }
            "open_url" -> {
                val url = action.url ?: return
                val intent = Intent(Intent.ACTION_VIEW, android.net.Uri.parse(url))
                activity.startActivity(intent)
            }
            "open_system_settings" -> {
                val intent = Intent(android.provider.Settings.ACTION_SETTINGS)
                activity.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                deliverCallbackJson("""{"kind":"settings_opened"}""")
            }
            "launch_app" -> {
                val packageName = action.packageName ?: return
                val intent = activity.packageManager.getLaunchIntentForPackage(packageName)
                if (intent == null) {
                    deliverCallbackJson(
                        """{"kind":"launch_app_failed","package":${org.json.JSONObject.quote(packageName)},"message":"no launch intent"}"""
                    )
                    return
                }
                activity.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                deliverCallbackJson(
                    """{"kind":"launch_app_started","package":${org.json.JSONObject.quote(packageName)}}"""
                )
            }
            else -> Unit
        }
    }

    companion object {
        fun create(
            activity: ComponentActivity,
            bindings: NativeBridgeBindings,
            termuxPendingIntentFactory: (String) -> PendingIntent,
        ): DeepSeekMobileHostCoordinator {
            return DeepSeekMobileHostCoordinator(
                activity = activity,
                bindings = bindings,
                documentPickerBridge = DeepSeekDocumentPickerBridge(
                    activity = activity,
                    callback = object : DeepSeekDocumentPickerBridge.Callback {
                        override fun onDocumentPickerPicked(result: AndroidDocumentPickerPickedResult) {
                            bindings.deliverHostCallbackJson(
                                """{"kind":"document_picker_picked","callback":${result.toJson()}}"""
                            )
                        }

                        override fun onDocumentPickerCancelled(requestId: String) {
                            bindings.deliverHostCallbackJson(
                                """{"kind":"document_picker_cancelled","request_id":"$requestId"}"""
                            )
                        }

                        override fun onDocumentPickerFailed(requestId: String, message: String) {
                            bindings.deliverHostCallbackJson(
                                """{"kind":"document_picker_failed","request_id":"$requestId","message":${org.json.JSONObject.quote(message)}}"""
                            )
                        }
                    },
                ),
                discoveryBridge = DeepSeekPcGatewayDiscoveryBridge(
                    context = activity,
                    callback = object : DeepSeekPcGatewayDiscoveryBridge.Callback {
                        override fun onPcGatewayDiscoveryStarted(requestId: String, serviceType: String) {
                            bindings.deliverHostCallbackJson(
                                """{"kind":"pc_discovery","callback":{"type":"started","request_id":"$requestId","service_type":"$serviceType"}}"""
                            )
                        }

                        override fun onPcGatewayDiscoveryCandidate(
                            requestId: String,
                            record: AndroidPcGatewayMdnsRecordPayload,
                        ) {
                            bindings.deliverHostCallbackJson(
                                """{"kind":"pc_discovery","callback":{"type":"updated","request_id":"$requestId","record":${record.toJson()}}}"""
                            )
                        }

                        override fun onPcGatewayDiscoveryCompleted(
                            requestId: String,
                            records: List<AndroidPcGatewayMdnsRecordPayload>,
                        ) {
                            val array = records.joinToString(prefix = "[", postfix = "]") { it.toJson() }
                            bindings.deliverHostCallbackJson(
                                """{"kind":"pc_discovery","callback":{"type":"completed","request_id":"$requestId","records":$array}}"""
                            )
                        }

                        override fun onPcGatewayDiscoveryFailed(requestId: String, message: String) {
                            bindings.deliverHostCallbackJson(
                                """{"kind":"pc_discovery","callback":{"type":"failed","request_id":"$requestId","message":${org.json.JSONObject.quote(message)}}}"""
                            )
                        }
                    },
                ),
                termuxBridge = DeepSeekTermuxBridge(activity),
                termuxPendingIntentFactory = termuxPendingIntentFactory,
            )
        }
    }
}

/** Implemented by the Dioxus Android shell (JNI or future interop). */
interface NativeBridgeBindings {
    fun pollNextHostActionJson(): String?
    fun deliverHostCallbackJson(payload: String)
}

private data class AndroidHostActionJson(
    val kind: String,
    val requestId: String? = null,
    val androidAction: String? = null,
    val androidCategory: String? = null,
    val mimeTypes: List<String>? = null,
    val allowMultiple: Boolean? = null,
    val serviceType: String? = null,
    val command: String? = null,
    val workingDir: String? = null,
    val path: String? = null,
    val mimeType: String? = null,
    val url: String? = null,
    val packageName: String? = null,
) {
    companion object {
        fun parse(raw: String): AndroidHostActionJson? {
            return try {
                val json = org.json.JSONObject(raw)
                AndroidHostActionJson(
                    kind = json.getString("kind"),
                    requestId = json.optString("request_id").ifBlank { null },
                    androidAction = json.optString("action").ifBlank { null },
                    androidCategory = json.optString("category").ifBlank { null },
                    mimeTypes = json.optJSONArray("mime_types")?.let { array ->
                        (0 until array.length()).map { idx -> array.getString(idx) }
                    },
                    allowMultiple = if (json.has("allow_multiple")) json.getBoolean("allow_multiple") else null,
                    serviceType = json.optString("service_type").ifBlank { null },
                    command = json.optString("command").ifBlank { null },
                    workingDir = json.optString("working_dir").ifBlank { null },
                    path = json.optString("path").ifBlank { null },
                    mimeType = json.optString("mime_type").ifBlank { null },
                    url = json.optString("url").ifBlank { null },
                    packageName = json.optString("package").ifBlank { null },
                )
            } catch (_: Throwable) {
                null
            }
        }
    }
}

private fun AndroidDocumentPickerPickedResult.toJson(): String {
    val docs = documents.joinToString(prefix = "[", postfix = "]") { doc ->
        org.json.JSONObject()
            .put("id", doc.id)
            .put("display_name", doc.displayName)
            .put("uri", doc.uri)
            .put("local_path", doc.localPath)
            .put("mime_type", doc.mimeType)
            .put("size_bytes", doc.sizeBytes)
            .toString()
    }
    return org.json.JSONObject()
        .put("type", "picked")
        .put("request_id", requestId)
        .put("documents", org.json.JSONArray(docs))
        .toString()
}

private fun AndroidPcGatewayMdnsRecordPayload.toJson(): String {
    return org.json.JSONObject()
        .put("instance_name", instanceName)
        .put("host", host)
        .put("port", port)
        .put("txt", org.json.JSONObject(txt))
        .toString()
}
