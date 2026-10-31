# Add project specific ProGuard rules here.
# By default, the flags in this file are appended to flags specified
# in Flutter's gradle rules.

# The ADB bridge helper is reached only by name: app_process names the starter
# class on the command line, and the helper's binder is reached through the
# AIDL stub. Keep them even if minification is ever turned on.
-keep class ee.nekoko.nlpa2.AdbBridgeStarter { *; }
-keep class ee.nekoko.nlpa2.AdbBridgeServer { *; }
-keep class ee.nekoko.nlpa2.IAdbBridge { *; }
-keep class ee.nekoko.nlpa2.IAdbBridge$* { *; }

# Shizuku API bindings
-keep class rikka.shizuku.** { *; }
-keep class dev.rikka.shizuku.** { *; }
-keep interface rikka.shizuku.** { *; }
-keep interface dev.rikka.shizuku.** { *; }

# Flutter Rust Bridge & Cargokit native JNI entrypoints
-keepclasseswithmembernames class * {
    native <methods>;
}
-keep class com.cargokit.** { *; }
-keep class flutter_rust_bridge.** { *; }

# Desugaring and core AndroidX
-dontwarn java.awt.**
-dontwarn javax.annotation.**
-keepattributes *Annotation*,Signature,InnerClasses,EnclosingMethod
