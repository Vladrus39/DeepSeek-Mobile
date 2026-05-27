package com.deepseek.mobile.bridge

import android.app.Activity
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import java.io.File
import java.io.IOException
import java.util.UUID
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/**
 * Native Android implementation of the Rust-side AndroidDocumentPickerCommand contract.
 *
 * Responsibilities:
 * - open ACTION_OPEN_DOCUMENT through ActivityResultContracts.StartActivityForResult;
 * - support single and multiple selection;
 * - copy every selected content:// Uri into app-private sandbox storage;
 * - return stable payloads with requestId, uri, localPath, mimeType and sizeBytes;
 * - keep the callback requestId identical to the command requestId so Rust can reject stale callbacks.
 */
class DeepSeekDocumentPickerBridge(
    private val activity: ComponentActivity,
    private val callback: Callback
) {
    interface Callback {
        fun onDocumentPickerPicked(result: AndroidDocumentPickerPickedResult)
        fun onDocumentPickerCancelled(requestId: String)
        fun onDocumentPickerFailed(requestId: String, message: String)
    }

    private var activeCommand: AndroidDocumentPickerCommandPayload? = null
    private val resultExecutor: ExecutorService = Executors.newSingleThreadExecutor { runnable ->
        Thread(runnable, "DeepSeekDocumentPicker").apply { isDaemon = true }
    }

    private val launcher: ActivityResultLauncher<Intent> =
        activity.registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            val command = activeCommand
            if (command == null) {
                callback.onDocumentPickerFailed(
                    requestId = "unknown",
                    message = "Document picker returned without an active command"
                )
                return@registerForActivityResult
            }

            activeCommand = null

            resultExecutor.execute {
                if (result.resultCode != Activity.RESULT_OK) {
                    callback.onDocumentPickerCancelled(command.requestId)
                    return@execute
                }

                val data = result.data
                if (data == null) {
                    callback.onDocumentPickerPicked(
                        AndroidDocumentPickerPickedResult(
                            requestId = command.requestId,
                            documents = emptyList()
                        )
                    )
                    return@execute
                }

                try {
                    val uris = selectedUris(data)
                    val documents = uris.map { uri -> copyUriToSandbox(command.requestId, uri) }
                    callback.onDocumentPickerPicked(
                        AndroidDocumentPickerPickedResult(
                            requestId = command.requestId,
                            documents = documents
                        )
                    )
                } catch (error: Throwable) {
                    callback.onDocumentPickerFailed(
                        requestId = command.requestId,
                        message = error.message ?: error.javaClass.simpleName
                    )
                }
            }
        }

    fun open(command: AndroidDocumentPickerCommandPayload) {
        activeCommand = command
        val intent = Intent(command.action).apply {
            addCategory(command.category)
            putExtra(Intent.EXTRA_ALLOW_MULTIPLE, command.allowMultiple)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)

            val mimeTypes = command.mimeTypes.ifEmpty { listOf("*/*") }
            type = if (mimeTypes.size == 1) mimeTypes.first() else "*/*"
            putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes.toTypedArray())
        }
        launcher.launch(intent)
    }

    fun shutdown() {
        resultExecutor.shutdownNow()
    }

    private fun selectedUris(data: Intent): List<Uri> {
        val clipData: ClipData? = data.clipData
        if (clipData != null && clipData.itemCount > 0) {
            return (0 until clipData.itemCount).mapNotNull { index -> clipData.getItemAt(index).uri }
        }
        return data.data?.let { listOf(it) } ?: emptyList()
    }

    private fun copyUriToSandbox(requestId: String, uri: Uri): AndroidPickedDocumentPayload {
        val resolver = activity.applicationContext.contentResolver
        val displayName = queryDisplayName(activity.applicationContext, uri)
            ?: uri.lastPathSegment
            ?: "attachment"
        val safeName = safeFileName(displayName)
        val mimeType = resolver.getType(uri)
        val outputDir = File(activity.filesDir, "attachments/$requestId")
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw IOException("Cannot create attachment sandbox directory: ${outputDir.absolutePath}")
        }
        val outputFile = uniqueOutputFile(outputDir, safeName)

        resolver.openInputStream(uri).use { input ->
            if (input == null) {
                throw IOException("Cannot open input stream for URI: $uri")
            }
            outputFile.outputStream().use { output -> input.copyTo(output) }
        }

        tryTakePersistableReadPermission(uri)

        return AndroidPickedDocumentPayload(
            id = UUID.randomUUID().toString(),
            displayName = displayName,
            uri = uri.toString(),
            localPath = outputFile.absolutePath,
            mimeType = mimeType,
            sizeBytes = outputFile.length()
        )
    }

    private fun tryTakePersistableReadPermission(uri: Uri) {
        try {
            activity.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION
            )
        } catch (_: Throwable) {
            // Some providers do not support persistable permissions. The sandbox copy is still enough
            // for Rust-side prompt ingestion, so this is intentionally non-fatal.
        }
    }
}

data class AndroidDocumentPickerCommandPayload(
    val requestId: String,
    val action: String = Intent.ACTION_OPEN_DOCUMENT,
    val category: String = Intent.CATEGORY_OPENABLE,
    val allowMultiple: Boolean = true,
    val mimeTypes: List<String> = listOf("*/*")
)

data class AndroidPickedDocumentPayload(
    val id: String,
    val displayName: String,
    val uri: String?,
    val localPath: String?,
    val mimeType: String?,
    val sizeBytes: Long?
)

data class AndroidDocumentPickerPickedResult(
    val requestId: String,
    val documents: List<AndroidPickedDocumentPayload>
)

fun queryDisplayName(context: Context, uri: Uri): String? {
    val cursor: Cursor? = context.contentResolver.query(
        uri,
        arrayOf(OpenableColumns.DISPLAY_NAME),
        null,
        null,
        null
    )
    cursor.use {
        if (it != null && it.moveToFirst()) {
            val index = it.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index >= 0) {
                return it.getString(index)
            }
        }
    }
    return null
}

fun safeFileName(name: String): String {
    val cleaned = name
        .replace(Regex("[^A-Za-z0-9._-]"), "_")
        .trim('_')
    return cleaned.ifBlank { "attachment" }
}

fun uniqueOutputFile(directory: File, safeName: String): File {
    val base = safeName.substringBeforeLast('.', safeName)
    val ext = safeName.substringAfterLast('.', "")
    var candidate = File(directory, safeName)
    var index = 1
    while (candidate.exists()) {
        val numberedName = if (ext.isBlank()) {
            "${base}_${index}"
        } else {
            "${base}_${index}.${ext}"
        }
        candidate = File(directory, numberedName)
        index += 1
    }
    return candidate
}
