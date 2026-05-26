package com.deepseek.mobile.bridge

/**
 * JNI bridge to the Rust library bundled by the active Android shell.
 *
 * This class lives in the Android bridge module so both the standalone Gradle
 * shell and the Dioxus-generated Android host can use the same native bridge.
 */
object NativeBridge {
    init {
        loadNativeLibrary()
    }

    private fun loadNativeLibrary() {
        try {
            // Dioxus generates the Android native activity library as libmain.so.
            System.loadLibrary("main")
        } catch (dioxusError: UnsatisfiedLinkError) {
            try {
                // Keep the standalone Gradle shell usable if it links the crate
                // under its cargo package name.
                System.loadLibrary("deepseek_mobile")
            } catch (standaloneError: UnsatisfiedLinkError) {
                dioxusError.addSuppressed(standaloneError)
                throw dioxusError
            }
        }
    }

    @JvmStatic
    external fun initMobileDataDir(absolutePath: String)

    @JvmStatic
    external fun pollNextHostActionJson(): String?

    @JvmStatic
    external fun deliverHostCallbackJson(payload: String)

    @JvmStatic
    external fun hasPendingCommands(): Boolean
}
