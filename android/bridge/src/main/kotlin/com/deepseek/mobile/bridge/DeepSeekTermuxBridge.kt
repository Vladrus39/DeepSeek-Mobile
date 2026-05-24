package com.deepseek.mobile.bridge

import android.app.Activity
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Bundle

/**
 * Android adapter for Termux RUN_COMMAND intents.
 *
 * The bridge intentionally keeps Termux concerns in Android:
 * - it launches Termux's RunCommandService with a structured command payload;
 * - it requests background execution so stdout/stderr are returned separately;
 * - it parses the PendingIntent result bundle back into a stable payload for Rust.
 *
 * The host Activity/Application is still responsible for creating the PendingIntent
 * receiver and forwarding the parsed payload into the Rust bridge.
 */
class DeepSeekTermuxBridge(
    private val context: Context
) {
    fun buildRunCommandIntent(
        command: AndroidTermuxCommandPayload,
        resultPendingIntent: PendingIntent
    ): Intent {
        return Intent().apply {
            setClassName(TERMUX_PACKAGE_NAME, TERMUX_RUN_COMMAND_SERVICE)
            action = TERMUX_RUN_COMMAND_ACTION
            putExtra(EXTRA_COMMAND_PATH, command.commandPath)
            putExtra(EXTRA_ARGUMENTS, command.arguments.toTypedArray())
            putExtra(EXTRA_WORKDIR, command.workingDir)
            putExtra(EXTRA_BACKGROUND, command.background)
            putExtra(EXTRA_COMMAND_LABEL, command.label)
            putExtra(EXTRA_COMMAND_DESCRIPTION, command.description)
            putExtra(EXTRA_PENDING_INTENT, resultPendingIntent)
        }
    }

    fun run(
        command: AndroidTermuxCommandPayload,
        resultPendingIntent: PendingIntent
    ) {
        context.startService(buildRunCommandIntent(command, resultPendingIntent))
    }

    fun parseResult(
        requestId: String,
        resultIntent: Intent
    ): AndroidTermuxResultPayload {
        val bundle: Bundle? = resultIntent.getBundleExtra(EXTRA_PLUGIN_RESULT_BUNDLE)
        if (bundle == null) {
            return AndroidTermuxResultPayload(
                requestId = requestId,
                stdout = "",
                stderr = "",
                exitCode = null,
                timedOut = false,
                error = "Termux result bundle is missing"
            )
        }

        val errCode = bundle.getInt(EXTRA_PLUGIN_RESULT_BUNDLE_ERR, Activity.RESULT_OK)
        val errMessage = bundle.getString(EXTRA_PLUGIN_RESULT_BUNDLE_ERRMSG)
        return AndroidTermuxResultPayload(
            requestId = requestId,
            stdout = bundle.getString(EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT).orEmpty(),
            stderr = bundle.getString(EXTRA_PLUGIN_RESULT_BUNDLE_STDERR).orEmpty(),
            exitCode = if (bundle.containsKey(EXTRA_PLUGIN_RESULT_BUNDLE_EXIT_CODE)) {
                bundle.getInt(EXTRA_PLUGIN_RESULT_BUNDLE_EXIT_CODE)
            } else {
                null
            },
            timedOut = false,
            error = if (errCode == Activity.RESULT_OK) {
                null
            } else {
                errMessage ?: "Termux error code $errCode"
            }
        )
    }

    companion object {
        const val TERMUX_PACKAGE_NAME = "com.termux"
        const val TERMUX_RUN_COMMAND_SERVICE = "com.termux.app.RunCommandService"
        const val TERMUX_RUN_COMMAND_ACTION = "com.termux.RUN_COMMAND"

        const val EXTRA_COMMAND_PATH = "com.termux.RUN_COMMAND_PATH"
        const val EXTRA_ARGUMENTS = "com.termux.RUN_COMMAND_ARGUMENTS"
        const val EXTRA_WORKDIR = "com.termux.RUN_COMMAND_WORKDIR"
        const val EXTRA_BACKGROUND = "com.termux.RUN_COMMAND_BACKGROUND"
        const val EXTRA_COMMAND_LABEL = "com.termux.RUN_COMMAND_COMMAND_LABEL"
        const val EXTRA_COMMAND_DESCRIPTION = "com.termux.RUN_COMMAND_COMMAND_DESCRIPTION"
        const val EXTRA_PENDING_INTENT = "com.termux.RUN_COMMAND_PENDING_INTENT"

        const val EXTRA_PLUGIN_RESULT_BUNDLE = "result"
        const val EXTRA_PLUGIN_RESULT_BUNDLE_STDOUT = "stdout"
        const val EXTRA_PLUGIN_RESULT_BUNDLE_STDERR = "stderr"
        const val EXTRA_PLUGIN_RESULT_BUNDLE_EXIT_CODE = "exitCode"
        const val EXTRA_PLUGIN_RESULT_BUNDLE_ERR = "err"
        const val EXTRA_PLUGIN_RESULT_BUNDLE_ERRMSG = "errmsg"
    }
}

data class AndroidTermuxCommandPayload(
    val requestId: String,
    val commandPath: String,
    val arguments: List<String>,
    val workingDir: String,
    val background: Boolean = true,
    val label: String = "DeepSeek Mobile command",
    val description: String = "Runs an approved command from DeepSeek Mobile"
)

data class AndroidTermuxResultPayload(
    val requestId: String,
    val stdout: String,
    val stderr: String,
    val exitCode: Int?,
    val timedOut: Boolean,
    val error: String?
)
