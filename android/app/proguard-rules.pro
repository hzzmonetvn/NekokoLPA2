# Project-specific ProGuard / R8 rules for pure-Android NekokoLPA2.

# The ADB bridge helper is reached only by name: app_process names the starter
# class on the command line, and the helper's binder is reached through the
# AIDL stub. Keep them even when minification is enabled.
-keep class ee.nekoko.nlpa2.AdbBridgeStarter { *; }
-keep class ee.nekoko.nlpa2.AdbBridgeServer { *; }
-keep class ee.nekoko.nlpa2.IAdbBridge { *; }
-keep class ee.nekoko.nlpa2.IAdbBridge$* { *; }

# Shizuku API bindings & QRTR / SE HAL user service
-keep class rikka.shizuku.** { *; }
-keep class dev.rikka.shizuku.** { *; }
-keep interface rikka.shizuku.** { *; }
-keep interface dev.rikka.shizuku.** { *; }
-keep class ee.nekoko.nlpa2.QrtrUserService { *; }
-keep class ee.nekoko.nlpa2.IQrtrService { *; }
-keep class ee.nekoko.nlpa2.IQrtrService$* { *; }

# Rust JNI bridge, host callback interface, and WebView JavaScript interfaces
-keep class ee.nekoko.nlpa2.RustBridge { *; }
-keep interface ee.nekoko.nlpa2.NativeHostBridge { *; }
-keep class ee.nekoko.nlpa2.NativeHostBridgeImpl { *; }
-keep class ee.nekoko.nlpa2.MainActivity { *; }
-keep class ee.nekoko.nlpa2.MainActivity$* { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}
-keepclassmembers class * {
    @android.webkit.JavascriptInterface <methods>;
}

# Desugaring and core Java annotations
-dontwarn java.awt.**
-dontwarn javax.annotation.**
-dontwarn org.bouncycastle.**
-keepattributes *Annotation*,Signature,InnerClasses,EnclosingMethod,JavascriptInterface
