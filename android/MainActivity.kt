package dev.dioxus.main

import android.app.PendingIntent
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import com.deepseek.mobile.bridge.DeepSeekMobileHostCoordinator
import com.deepseek.mobile.bridge.NativeBridge
import com.deepseek.mobile.bridge.NativeBridgeBindings
import com.deepseek.mobile.bridge.TermuxResultReceiver
import java.io.File

/**
 * Dioxus 0.7 generates Kotlin support files in `dev.dioxus.main`, while Gradle
 * generates `BuildConfig` under the configured Android namespace. Keep a small
 * same-package shim so generated support code can resolve `BuildConfig.DEBUG`.
 */
object BuildConfig {
    @JvmField
    val DEBUG: Boolean = com.deepseek.mobile.BuildConfig.DEBUG
}

/**
 * Dioxus Android shell activity. Must subclass [WryActivity] so the mobile runtime can start.
 *
 * Host command drain runs on the main thread while the activity is resumed.
 */
class MainActivity : WryActivity(), NativeBridgeBindings {
    private lateinit var hostCoordinator: DeepSeekMobileHostCoordinator
    private val hostHandler = Handler(Looper.getMainLooper())
    private val drainRunnable = object : Runnable {
        override fun run() {
            while (hostCoordinator.pollAndHandleNextAction()) {
                // Drain the Rust native queue until empty.
            }
            hostHandler.postDelayed(this, HOST_POLL_INTERVAL_MS)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        val dataDir = File(filesDir, "deepseek-mobile")
        dataDir.mkdirs()
        NativeBridge.initMobileDataDir(dataDir.absolutePath)
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
