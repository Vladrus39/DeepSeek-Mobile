package com.deepseek.mobile

import android.app.PendingIntent
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import com.deepseek.mobile.bridge.DeepSeekMobileHostCoordinator
import com.deepseek.mobile.bridge.NativeBridgeBindings

/**
 * Standalone Gradle launcher (without Dioxus WebView). For production APK use
 * `dx build android` with [dev.dioxus.main.MainActivity].
 */
class MainActivity : ComponentActivity(), NativeBridgeBindings {
    private lateinit var hostCoordinator: DeepSeekMobileHostCoordinator
    private val hostHandler = Handler(Looper.getMainLooper())
    private val drainRunnable = object : Runnable {
        override fun run() {
            while (hostCoordinator.pollAndHandleNextAction()) {
                // Drain until the Rust queue is empty.
            }
            hostHandler.postDelayed(this, HOST_POLL_INTERVAL_MS)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        hostCoordinator = DeepSeekMobileHostCoordinator.create(
            activity = this,
            bindings = this,
            termuxPendingIntentFactory = { requestId ->
                PendingIntent.getBroadcast(
                    this,
                    requestId.hashCode(),
                    Intent(this, TermuxResultReceiver::class.java).apply {
                        putExtra(TermuxResultReceiver.EXTRA_REQUEST_ID, requestId)
                    },
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
                )
            },
        )
    }

    override fun onResume() {
        super.onResume()
        hostHandler.removeCallbacks(drainRunnable)
        hostHandler.post(drainRunnable)
    }

    override fun onPause() {
        hostHandler.removeCallbacks(drainRunnable)
        super.onPause()
    }

    override fun pollNextHostActionJson(): String? = NativeBridge.pollNextHostActionJson()

    override fun deliverHostCallbackJson(payload: String) {
        NativeBridge.deliverHostCallbackJson(payload)
    }

    companion object {
        private const val HOST_POLL_INTERVAL_MS = 250L
    }
}
