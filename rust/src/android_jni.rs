//! Pure-Rust bridge dispatch entry points and Android `ee.nekoko.nlpa2.RustBridge` JNI bindings.

use std::sync::{Arc, Mutex};

use nlpa_ui::RenderMeta;

use crate::controller::{AppConfig, AppController};
use crate::host::HostTransport;

static GLOBAL_CONTROLLER: Mutex<Option<AppController>> = Mutex::new(None);

/// Initializes the global [`AppController`] instance and returns the full initial HTML5 document.
pub fn init_controller(host: Arc<dyn HostTransport>, config: AppConfig) -> String {
    let controller = AppController::new(host, config);
    let html = controller.render_document();
    if let Ok(mut guard) = GLOBAL_CONTROLLER.lock() {
        *guard = Some(controller);
    }
    html
}

/// Dispatches an action JSON payload through the global [`AppController`] and returns `RenderMeta` JSON.
#[must_use]
pub fn dispatch_action_json(action_json: &str) -> String {
    let Ok(mut guard) = GLOBAL_CONTROLLER.lock() else {
        return uninitialized_render_json("Controller lock poisoned");
    };
    let Some(controller) = guard.as_mut() else {
        return uninitialized_render_json("Controller not initialized");
    };
    let meta = controller.dispatch_json(action_json);
    serialize_render_meta(&meta)
}

/// Renders the current global [`AppController`] state to `RenderMeta` JSON.
#[must_use]
pub fn render_state_json() -> String {
    let Ok(guard) = GLOBAL_CONTROLLER.lock() else {
        return uninitialized_render_json("Controller lock poisoned");
    };
    let Some(controller) = guard.as_ref() else {
        return uninitialized_render_json("Controller not initialized");
    };
    let meta = controller.render_meta();
    serialize_render_meta(&meta)
}

/// Delivers an asynchronous platform event JSON string to the global [`AppController`].
#[must_use]
pub fn handle_platform_event_json(event_json: &str) -> String {
    let Ok(mut guard) = GLOBAL_CONTROLLER.lock() else {
        return uninitialized_render_json("Controller lock poisoned");
    };
    let Some(controller) = guard.as_mut() else {
        return uninitialized_render_json("Controller not initialized");
    };
    let meta = controller.on_platform_event_json(event_json);
    serialize_render_meta(&meta)
}

/// Clears the global controller instance (intended for isolated integration tests).
pub fn reset_global_controller() {
    if let Ok(mut guard) = GLOBAL_CONTROLLER.lock() {
        *guard = None;
    }
}

fn serialize_render_meta(meta: &RenderMeta) -> String {
    serde_json::to_string(meta)
        .unwrap_or_else(|e| uninitialized_render_json(&format!("Render JSON encode failed: {e}")))
}

fn uninitialized_render_json(msg: &str) -> String {
    let meta = RenderMeta {
        html: format!("<div class=\"nlpa-shell is-dark is-amoled\"><div class=\"nlpa-main\"><div class=\"nlpa-banner is-danger\">{msg}</div></div></div>"),
        theme_style: "amoled".into(),
        dark: true,
        wallpaper_active: false,
        wallpaper_opacity: 0.35,
        status_bar_black: true,
    };
    serde_json::to_string(&meta).unwrap_or_else(|_| "{}".into())
}

#[cfg(target_os = "android")]
mod jni_exports {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::path::PathBuf;
    use std::sync::Arc;

    use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
    use jni::sys::{jboolean, jint, jstring, JNI_TRUE};
    use jni::{JNIEnv, JavaVM};

    use super::{
        dispatch_action_json, handle_platform_event_json, init_controller, render_state_json,
        uninitialized_render_json,
    };
    use crate::controller::AppConfig;
    use crate::host::{HostCallError, HostTransport};

    struct AndroidHostBridge {
        vm: JavaVM,
        host_ref: GlobalRef,
    }

    impl HostTransport for AndroidHostBridge {
        fn call_host_raw(&self, request_json: &str) -> Result<String, HostCallError> {
            let mut env = self
                .vm
                .attach_current_thread()
                .map_err(|e| HostCallError::new("JNI_ATTACH", e.to_string()))?;
            let j_req = env
                .new_string(request_json)
                .map_err(|e| HostCallError::new("JNI_STRING", e.to_string()))?;
            let res = env.call_method(
                self.host_ref.as_obj(),
                "callHost",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(j_req.as_ref())],
            );
            let value = match res {
                Ok(val) => val,
                Err(err) => {
                    let _ = env.exception_clear();
                    return Err(HostCallError::new("JNI_CALL", err.to_string()));
                }
            };
            let obj = value
                .l()
                .map_err(|e| HostCallError::new("JNI_RETURN", e.to_string()))?;
            if obj.is_null() {
                return Err(HostCallError::new(
                    "JNI_NULL_RETURN",
                    "NativeHostBridge.callHost returned null",
                ));
            }
            let j_resp = JString::from(obj);
            let java_str = env
                .get_string(&j_resp)
                .map_err(|e| HostCallError::new("JNI_DECODE", e.to_string()))?;
            let decoded: String = java_str.into();
            Ok(decoded)
        }

        fn notify_state_updated(&self) {
            let Ok(mut env) = self.vm.attach_current_thread() else {
                return;
            };
            if env
                .call_method(self.host_ref.as_obj(), "onStateUpdated", "()V", &[])
                .is_err()
            {
                let _ = env.exception_clear();
            }
        }
    }

    fn read_jstring(env: &mut JNIEnv<'_>, s: &JString<'_>) -> String {
        if s.is_null() {
            return String::new();
        }
        match env.get_string(s) {
            Ok(java_str) => java_str.into(),
            Err(_) => {
                let _ = env.exception_clear();
                String::new()
            }
        }
    }

    fn make_jstring<'local>(env: &mut JNIEnv<'local>, text: &str) -> jstring {
        match env.new_string(text) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    #[allow(clippy::too_many_arguments)]
    pub extern "system" fn Java_ee_nekoko_nlpa2_RustBridge_nativeInit<'local>(
        mut env: JNIEnv<'local>,
        _class: JClass<'local>,
        host: JObject<'local>,
        files_dir: JString<'local>,
        cache_dir: JString<'local>,
        package_name: JString<'local>,
        app_version: JString<'local>,
        version_code: jint,
        is_privileged: jboolean,
        system_locale: JString<'local>,
        entitlement_json: JString<'local>,
        reference_sizes_json: JString<'local>,
    ) -> jstring {
        let html = catch_unwind(AssertUnwindSafe(|| {
            let vm = match env.get_java_vm() {
                Ok(vm) => vm,
                Err(e) => return format!("<!DOCTYPE html><html><body>JNI VM error: {e}</body></html>"),
            };
            let host_ref = match env.new_global_ref(&host) {
                Ok(r) => r,
                Err(e) => return format!("<!DOCTYPE html><html><body>JNI GlobalRef error: {e}</body></html>"),
            };
            let config = AppConfig {
                files_dir: PathBuf::from(read_jstring(&mut env, &files_dir)),
                cache_dir: PathBuf::from(read_jstring(&mut env, &cache_dir)),
                package_name: read_jstring(&mut env, &package_name),
                app_version: read_jstring(&mut env, &app_version),
                version_code,
                is_privileged: is_privileged == JNI_TRUE,
                system_locale: read_jstring(&mut env, &system_locale),
                entitlement_json: read_jstring(&mut env, &entitlement_json),
                reference_sizes_json: read_jstring(&mut env, &reference_sizes_json),
            };
            let bridge = Arc::new(AndroidHostBridge { vm, host_ref });
            init_controller(bridge, config)
        }))
        .unwrap_or_else(|_| "<!DOCTYPE html><html><body>Initialization panic</body></html>".into());

        make_jstring(&mut env, &html)
    }

    #[no_mangle]
    pub extern "system" fn Java_ee_nekoko_nlpa2_RustBridge_nativeDispatch<'local>(
        mut env: JNIEnv<'local>,
        _class: JClass<'local>,
        action_json: JString<'local>,
    ) -> jstring {
        let out = catch_unwind(AssertUnwindSafe(|| {
            let raw = read_jstring(&mut env, &action_json);
            dispatch_action_json(&raw)
        }))
        .unwrap_or_else(|_| uninitialized_render_json("Dispatch panic caught"));

        make_jstring(&mut env, &out)
    }

    #[no_mangle]
    pub extern "system" fn Java_ee_nekoko_nlpa2_RustBridge_nativeRender<'local>(
        mut env: JNIEnv<'local>,
        _class: JClass<'local>,
    ) -> jstring {
        let out = catch_unwind(AssertUnwindSafe(render_state_json))
            .unwrap_or_else(|_| uninitialized_render_json("Render panic caught"));

        make_jstring(&mut env, &out)
    }

    #[no_mangle]
    pub extern "system" fn Java_ee_nekoko_nlpa2_RustBridge_nativeOnPlatformEvent<'local>(
        mut env: JNIEnv<'local>,
        _class: JClass<'local>,
        event_json: JString<'local>,
    ) -> jstring {
        let out = catch_unwind(AssertUnwindSafe(|| {
            let raw = read_jstring(&mut env, &event_json);
            handle_platform_event_json(&raw)
        }))
        .unwrap_or_else(|_| uninitialized_render_json("Platform event panic caught"));

        make_jstring(&mut env, &out)
    }
}
