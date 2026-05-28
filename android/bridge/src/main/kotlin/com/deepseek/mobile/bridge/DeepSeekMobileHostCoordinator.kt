package com.deepseek.mobile.bridge

import android.app.PendingIntent
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.core.content.FileProvider
import java.io.File
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

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
    private val callbackExecutor: ExecutorService,
    private val onExternalActivityLaunched: () -> Unit,
) {
    fun pollAndHandleNextAction(): Boolean {
        val actionJson = bindings.pollNextHostActionJson() ?: return false
        handleActionJson(actionJson)
        return true
    }

    fun deliverCallbackJson(payload: String) {
        activity.runOnUiThread {
            bindings.deliverHostCallbackJson(payload)
        }
    }

    fun shutdown() {
        documentPickerBridge.shutdown()
        callbackExecutor.shutdownNow()
    }

    fun handleActionJson(actionJson: String) {
        val action = AndroidHostActionJson.parse(actionJson) ?: return
        when (action.kind) {
            "open_document_picker" -> {
                val requestId = action.requestId ?: return
                val androidAction = action.androidAction ?: Intent.ACTION_OPEN_DOCUMENT
                val androidCategory = action.androidCategory ?: Intent.CATEGORY_OPENABLE
                val allowMultiple = action.allowMultiple ?: false
                val mimeTypes = action.mimeTypes ?: listOf("*/*")
                val command = AndroidDocumentPickerCommandPayload(
                    requestId = requestId,
                    action = androidAction,
                    category = androidCategory,
                    allowMultiple = allowMultiple,
                    mimeTypes = mimeTypes,
                )
                activity.runOnUiThread {
                    try {
                        onExternalActivityLaunched()
                        documentPickerBridge.open(command)
                    } catch (e: Throwable) {
                        deliverCallbackJson(
                            """{"kind":"document_picker_failed","request_id":"$requestId","message":${org.json.JSONObject.quote(e.message ?: "document picker launch failed")}}"""
                        )
                    }
                }
            }
            "start_pc_gateway_discovery" -> {
                val requestId = action.requestId ?: return
                val serviceType = action.serviceType ?: "_deepseek-pc-gateway._tcp."
                val timeoutMs = action.timeoutMs ?: 12_000L
                // NSD must run on the main looper; host drain uses a background executor.
                activity.runOnUiThread {
                    Log.i(TAG, "Starting PC gateway NSD discovery requestId=$requestId")
                    discoveryBridge.startDiscovery(
                        AndroidPcGatewayDiscoveryCommandPayload(
                            requestId = requestId,
                            serviceType = serviceType,
                            timeoutMs = timeoutMs,
                        ),
                    )
                }
            }
            "run_termux_command" -> {
                val requestId = action.requestId ?: return
                val command = action.command ?: return
                val workingDir = action.workingDir ?: return
                val pending = termuxPendingIntentFactory(requestId)
                val payload = AndroidTermuxCommandPayload(
                    requestId = requestId,
                    commandPath = "/data/data/com.termux/files/usr/bin/sh",
                    arguments = listOf("-lc", command),
                    workingDir = workingDir,
                    background = true,
                    label = "DeepSeek Mobile",
                    description = command,
                )
                val startError = termuxBridge.run(payload, pending)
                if (startError != null) {
                    deliverCallbackJson(
                        """{"kind":"termux_failed","request_id":"$requestId","message":${org.json.JSONObject.quote(startError)}}""",
                    )
                }
            }
            "install_apk" -> {
                val path = action.path ?: return
                activity.runOnUiThread {
                    val file = File(path)
                    if (!file.exists()) {
                        deliverCallbackJson(
                            """{"kind":"apk_install_failed","message":"file not found: $path"}""",
                        )
                        return@runOnUiThread
                    }
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                        if (!activity.packageManager.canRequestPackageInstalls()) {
                            val settingsIntent =
                                Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES).apply {
                                    data = Uri.parse("package:${activity.packageName}")
                                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                }
                            try {
                                onExternalActivityLaunched()
                                activity.startActivity(settingsIntent)
                            } catch (e: Throwable) {
                                deliverCallbackJson(
                                    """{"kind":"apk_install_failed","message":${org.json.JSONObject.quote(e.message ?: "unknown sources settings failed")}}""",
                                )
                                return@runOnUiThread
                            }
                            deliverCallbackJson("""{"kind":"apk_install_needs_permission"}""")
                            return@runOnUiThread
                        }
                    }
                    val authority = "${activity.packageName}.fileprovider"
                    try {
                        val uri = FileProvider.getUriForFile(activity, authority, file)
                        val install =
                            Intent(Intent.ACTION_VIEW).apply {
                                setDataAndType(uri, "application/vnd.android.package-archive")
                                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                            }
                        onExternalActivityLaunched()
                        activity.startActivity(install)
                        deliverCallbackJson("""{"kind":"apk_install_started"}""")
                    } catch (e: Throwable) {
                        deliverCallbackJson(
                            """{"kind":"apk_install_failed","message":${org.json.JSONObject.quote(e.message ?: "apk install launch failed")}}""",
                        )
                    }
                }
            }
            "share_file" -> {
                val path = action.path ?: return
                val mimeType = action.mimeType ?: "application/octet-stream"
                activity.runOnUiThread {
                    val file = File(path)
                    if (!file.exists()) {
                        deliverCallbackJson(
                            """{"kind":"share_failed","message":"file not found: $path"}""",
                        )
                        return@runOnUiThread
                    }
                    val authority = "${activity.packageName}.fileprovider"
                    try {
                        val uri = FileProvider.getUriForFile(activity, authority, file)
                        val send =
                            Intent(Intent.ACTION_SEND).apply {
                                type = mimeType
                                putExtra(Intent.EXTRA_STREAM, uri)
                                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                            }
                        val matchFlags =
                            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                                PackageManager.MATCH_DEFAULT_ONLY
                            } else {
                                0
                            }
                        val targets =
                            activity.packageManager.queryIntentActivities(send, matchFlags)
                        for (target in targets) {
                            val pkgName = target.activityInfo?.packageName ?: continue
                            activity.grantUriPermission(
                                pkgName,
                                uri,
                                Intent.FLAG_GRANT_READ_URI_PERMISSION,
                            )
                        }
                        val chooser =
                            Intent.createChooser(send, "Share file").apply {
                                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                            }
                        onExternalActivityLaunched()
                        activity.startActivity(chooser)
                        deliverCallbackJson("""{"kind":"file_shared"}""")
                    } catch (e: Throwable) {
                        deliverCallbackJson(
                            """{"kind":"share_failed","message":${org.json.JSONObject.quote(e.message ?: "share failed")}}""",
                        )
                    }
                }
            }
            "open_url" -> {
                val url = action.url ?: return
                val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url)).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                return try {
                    onExternalActivityLaunched()
                    activity.startActivity(intent)
                    deliverCallbackJson("""{"kind":"url_opened","url":${org.json.JSONObject.quote(url)}}""")
                } catch (e: Exception) {
                    deliverCallbackJson(
                        """{"kind":"url_open_failed","url":${org.json.JSONObject.quote(url)},"message":${org.json.JSONObject.quote(e.message ?: "open failed")}}""",
                    )
                }
            }
            "open_system_settings" -> {
                val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                    data = Uri.fromParts("package", activity.packageName, null)
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                return try {
                    onExternalActivityLaunched()
                    activity.startActivity(intent)
                    deliverCallbackJson("""{"kind":"system_settings_opened"}""")
                } catch (e: Exception) {
                    deliverCallbackJson(
                        """{"kind":"system_settings_failed","message":${org.json.JSONObject.quote(e.message ?: "settings failed")}}""",
                    )
                }
            }
            "launch_app" -> {
                val pkg = action.packageName ?: return
                val launch = activity.packageManager.getLaunchIntentForPackage(pkg)
                if (launch != null) {
                    return try {
                        onExternalActivityLaunched()
                        launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                        activity.startActivity(launch)
                        deliverCallbackJson("""{"kind":"app_launched","package":"$pkg"}""")
                    } catch (e: Exception) {
                        deliverCallbackJson(
                            """{"kind":"app_launch_failed","package":"$pkg","message":${org.json.JSONObject.quote(e.message ?: "launch failed")}}""",
                        )
                    }
                }
                val market = Intent(Intent.ACTION_VIEW, Uri.parse("https://f-droid.org/packages/$pkg/")).apply {
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                return try {
                    onExternalActivityLaunched()
                    activity.startActivity(market)
                    deliverCallbackJson("""{"kind":"app_store_opened","package":"$pkg"}""")
                } catch (e: Exception) {
                    deliverCallbackJson(
                        """{"kind":"app_launch_failed","package":"$pkg","message":${org.json.JSONObject.quote(e.message ?: "store open failed")}}""",
                    )
                }
            }
            "request_termux_permission" -> {
                val pkg = DeepSeekTermuxBridge.TERMUX_PACKAGE_NAME
                val granted = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    activity.checkSelfPermission("com.termux.permission.RUN_COMMAND") == PackageManager.PERMISSION_GRANTED
                } else {
                    true
                }
                deliverCallbackJson(
                    """{"kind":"termux_permission_status","package":"$pkg","granted":$granted}""",
                )
            }
            else -> Unit
        }
    }

    companion object {
        private const val TAG = "DeepSeekHostCoordinator"

        /** Binder/NSD/Termux threads must not call JNI directly; Wry/Dioxus expects the UI thread. */
        private fun postHostCallbackOnUiThread(
            activity: ComponentActivity,
            bindings: NativeBridgeBindings,
            payload: String,
        ) {
            activity.runOnUiThread {
                bindings.deliverHostCallbackJson(payload)
            }
        }

        fun create(
            activity: ComponentActivity,
            bindings: NativeBridgeBindings,
            onExternalActivityLaunched: () -> Unit,
            termuxPendingIntentFactory: (String) -> PendingIntent,
        ): DeepSeekMobileHostCoordinator {
            val callbackExecutor = Executors.newSingleThreadExecutor { runnable ->
                Thread(runnable, "DeepSeekNativeCallbacks").apply { isDaemon = true }
            }
            return DeepSeekMobileHostCoordinator(
                activity = activity,
                bindings = bindings,
                documentPickerBridge = DeepSeekDocumentPickerBridge(
                    activity = activity,
                    callback = object : DeepSeekDocumentPickerBridge.Callback {
                        override fun onDocumentPickerPicked(result: AndroidDocumentPickerPickedResult) {
                            postHostCallbackOnUiThread(
                                activity,
                                bindings,
                                """{"kind":"document_picker_picked","callback":${result.toJson()}}""",
                            )
                        }

                        override fun onDocumentPickerCancelled(requestId: String) {
                            postHostCallbackOnUiThread(
                                activity,
                                bindings,
                                """{"kind":"document_picker_cancelled","request_id":"$requestId"}""",
                            )
                        }

                        override fun onDocumentPickerFailed(requestId: String, message: String) {
                            postHostCallbackOnUiThread(
                                activity,
                                bindings,
                                """{"kind":"document_picker_failed","request_id":"$requestId","message":${org.json.JSONObject.quote(message)}}""",
                            )
                        }
                    },
                ),
                discoveryBridge = DeepSeekPcGatewayDiscoveryBridge(
                    context = activity,
                    callback = object : DeepSeekPcGatewayDiscoveryBridge.Callback {
                        override fun onPcGatewayDiscoveryStarted(requestId: String, serviceType: String) {
                            postHostCallbackOnUiThread(
                                activity,
                                bindings,
                                """{"kind":"pc_discovery","callback":{"type":"started","request_id":"$requestId","service_type":"$serviceType"}}""",
                            )
                        }

                        override fun onPcGatewayDiscoveryCandidate(
                            requestId: String,
                            record: AndroidPcGatewayMdnsRecordPayload,
                        ) {
                            postHostCallbackOnUiThread(
                                activity,
                                bindings,
                                """{"kind":"pc_discovery","callback":{"type":"updated","request_id":"$requestId","record":${record.toJson()}}}""",
                            )
                        }

                        override fun onPcGatewayDiscoveryCompleted(
                            requestId: String,
                            records: List<AndroidPcGatewayMdnsRecordPayload>,
                        ) {
                            val array = records.joinToString(prefix = "[", postfix = "]") { it.toJson() }
                            postHostCallbackOnUiThread(
                                activity,
                                bindings,
                                """{"kind":"pc_discovery","callback":{"type":"completed","request_id":"$requestId","records":$array}}""",
                            )
                        }

                        override fun onPcGatewayDiscoveryFailed(requestId: String, message: String) {
                            postHostCallbackOnUiThread(
                                activity,
                                bindings,
                                """{"kind":"pc_discovery","callback":{"type":"failed","request_id":"$requestId","message":${org.json.JSONObject.quote(message)}}}""",
                            )
                        }
                    },
                ),
                termuxBridge = DeepSeekTermuxBridge(activity),
                termuxPendingIntentFactory = termuxPendingIntentFactory,
                callbackExecutor = callbackExecutor,
                onExternalActivityLaunched = onExternalActivityLaunched,
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
    val timeoutMs: Long? = null,
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
                    timeoutMs = if (json.has("timeout_ms")) json.optLong("timeout_ms") else null,
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
