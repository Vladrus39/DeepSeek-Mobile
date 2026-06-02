package dev.dioxus.main

import android.app.PendingIntent
import android.content.Intent
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.view.View
import androidx.core.view.WindowCompat
import com.deepseek.mobile.bridge.DeepSeekMobileHostCoordinator
import com.deepseek.mobile.bridge.NativeBridge
import com.deepseek.mobile.bridge.NativeBridgeBindings
import com.deepseek.mobile.bridge.TermuxResultReceiver
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit

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
 * Host command drain runs while the activity process is alive (not only when resumed)
 * so Termux RUN_COMMAND can flush when the WebView pauses.
 */
class MainActivity : WryActivity(), NativeBridgeBindings {
    private lateinit var hostCoordinator: DeepSeekMobileHostCoordinator
    private val hostExecutor = Executors.newSingleThreadScheduledExecutor { runnable ->
        Thread(runnable, "DeepSeekHostDrain").apply { isDaemon = true }
    }
    private var hostDrainFuture: ScheduledFuture<*>? = null
    private val drainRunnable = Runnable {
        try {
            var drained = 0
            while (drained < HOST_MAX_ACTIONS_PER_TICK && hostCoordinator.pollAndHandleNextAction()) {
                drained += 1
            }
        } catch (error: Throwable) {
            Log.e(TAG, "Native host drain failed", error)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        val dataDir = File(filesDir, "deepseek-mobile")
        dataDir.mkdirs()
        NativeBridge.initMobileDataDir(dataDir.absolutePath)
        super.onCreate(savedInstanceState)

        // Proactively request any standard runtime permissions early so the user
        // sees the system dialogs as soon as possible after first launch.
        // Note: com.termux.permission.RUN_COMMAND is special — Termux itself shows
        // the allow dialog the first time we send a RUN_COMMAND intent (we trigger
        // it from the setup "Test RUN_COMMAND" button + provisioning probe).
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val toRequest = mutableListOf<String>()
            if (checkSelfPermission(android.Manifest.permission.INTERNET) != android.content.pm.PackageManager.PERMISSION_GRANTED) {
                toRequest += android.Manifest.permission.INTERNET
            }
            // Add other runtime permissions here if we declare more dangerous ones in the future.
            if (toRequest.isNotEmpty()) {
                requestPermissions(toRequest.toTypedArray(), 1001)
            }
        }

        configureEdgeToEdgeShell()
        hostCoordinator = DeepSeekMobileHostCoordinator.create(
            activity = this,
            bindings = this,
            onExternalActivityLaunched = {},
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
        startHostDrain()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        // Dioxus/Wry forwards this event into native code. On the tested Samsung
        // Android 13 device, returning from ACTION_OPEN_DOCUMENT can hang while
        // Android waits for FocusEvent(hasFocus=true). The WebView remains
        // interactive without the native focus callback, so we intentionally do
        // not call super here.
    }

    override fun onResume() {
        super.onResume()
        startHostDrain()
    }

    override fun onPause() {
        // Keep draining the Rust host queue in background; calibration/Termux must not
        // stall when Android pauses the WebView (e.g. split-screen, brief overlays).
        super.onPause()
    }

    override fun onDestroy() {
        stopHostDrain()
        if (::hostCoordinator.isInitialized) {
            hostCoordinator.shutdown()
        }
        hostExecutor.shutdownNow()
        super.onDestroy()
    }

    override fun pollNextHostActionJson(): String? = NativeBridge.pollNextHostActionJson()

    override fun deliverHostCallbackJson(payload: String) {
        NativeBridge.deliverHostCallbackJson(payload)
    }

    private fun configureEdgeToEdgeShell() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        window.statusBarColor = Color.TRANSPARENT
        window.navigationBarColor = Color.BLACK
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility =
            View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
                View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or
                View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
    }

    private fun startHostDrain() {
        if (!::hostCoordinator.isInitialized) return
        if (hostDrainFuture?.isCancelled == false && hostDrainFuture?.isDone == false) return
        hostDrainFuture = hostExecutor.scheduleWithFixedDelay(
            drainRunnable,
            0L,
            HOST_POLL_INTERVAL_MS,
            TimeUnit.MILLISECONDS,
        )
    }

    private fun stopHostDrain() {
        hostDrainFuture?.cancel(false)
        hostDrainFuture = null
    }

    companion object {
        private const val HOST_POLL_INTERVAL_MS = 250L
        private const val HOST_MAX_ACTIONS_PER_TICK = 16
        private const val TAG = "DeepSeekMainActivity"
    }
}
