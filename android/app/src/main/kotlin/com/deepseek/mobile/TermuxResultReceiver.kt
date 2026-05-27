package com.deepseek.mobile

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.Looper
import com.deepseek.mobile.bridge.DeepSeekTermuxBridge
import org.json.JSONObject
import java.io.File

class TermuxResultReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        com.deepseek.mobile.bridge.NativeBridge.initMobileDataDir(
            File(context.filesDir, "deepseek-mobile").absolutePath,
        )
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
        Handler(Looper.getMainLooper()).post {
            NativeBridge.deliverHostCallbackJson(payload)
        }
    }

    companion object {
        const val EXTRA_REQUEST_ID = "request_id"
    }
}
