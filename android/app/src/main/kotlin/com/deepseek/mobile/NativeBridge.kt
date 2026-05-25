package com.deepseek.mobile

/**
 * JNI bridge to the Rust `deepseek_mobile` library.
 */
object NativeBridge {
    init {
        System.loadLibrary("deepseek_mobile")
    }

    @JvmStatic
    external fun pollNextHostActionJson(): String?

    @JvmStatic
    external fun deliverHostCallbackJson(payload: String)

    @JvmStatic
    external fun hasPendingCommands(): Boolean
}
