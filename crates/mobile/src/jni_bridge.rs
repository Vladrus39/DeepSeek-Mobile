//! JNI exports for the Android Dioxus shell (`com.deepseek.mobile.bridge.NativeBridge`).

use crate::mobile_data_dir;
use crate::native_host_runtime;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;

fn java_string(env: &mut JNIEnv, value: Option<String>) -> jstring {
    match value {
        Some(text) => env
            .new_string(text)
            .map(|string| string.into_raw())
            .unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_deepseek_mobile_bridge_NativeBridge_initMobileDataDir(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) {
    let Ok(path) = env.get_string(&path) else {
        return;
    };
    mobile_data_dir::set_mobile_data_dir(path.to_string_lossy().to_string());
}

#[no_mangle]
pub extern "system" fn Java_com_deepseek_mobile_bridge_NativeBridge_pollNextHostActionJson(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    java_string(&mut env, native_host_runtime::poll_next_host_action_json())
}

#[no_mangle]
pub extern "system" fn Java_com_deepseek_mobile_bridge_NativeBridge_deliverHostCallbackJson(
    mut env: JNIEnv,
    _class: JClass,
    payload: JString,
) {
    let Ok(payload) = env.get_string(&payload) else {
        return;
    };
    let _ = native_host_runtime::deliver_host_callback_json(&payload.to_string_lossy());
}

#[no_mangle]
pub extern "system" fn Java_com_deepseek_mobile_bridge_NativeBridge_hasPendingCommands(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jboolean {
    native_host_runtime::has_pending_commands().into()
}
