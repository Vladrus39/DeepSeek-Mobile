package com.deepseek.mobile.bridge

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import org.json.JSONObject

/**
 * Receives Termux `RUN_COMMAND` results and forwards them into the Rust bridge.
 */
class TermuxResultReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val requestId = intent.getStringExtra(EXTRA_REQUEST_ID) ?: return
        val result = DeepSeekTermuxBridge(context).parseResult(requestId, intent)
        val payload = JSONObject()
            .put("kind", "termux_completed")
            .put(
                "result",
                JSONObject()
                    .put("request_id", result.requestId)
                    .put("stdout", result.stdout)
                    .put("stderr", result.stderr)
                    .put("exit_code", result.exitCode)
                    .put("timed_out", result.timedOut)
                    .put("error", result.error),
            )
            .toString()
        NativeBridge.deliverHostCallbackJson(payload)
    }

    companion object {
        const val EXTRA_REQUEST_ID = "request_id"
    }
}
