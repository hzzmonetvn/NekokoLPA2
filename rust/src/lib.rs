//! Pure-Rust native library (`rust_lib_nlpa2`) combining `nlpa_core`, `nlpa_ui`,
//! `AppController`, and Android JNI bindings for `ee.nekoko.nlpa2.RustBridge`.

pub mod android_jni;
pub mod controller;
pub mod host;
pub mod lpa_ops;
pub mod readers;
pub mod runtime;
pub mod ts43_ops;

#[cfg(target_os = "android")]
mod qrtr_bus;
