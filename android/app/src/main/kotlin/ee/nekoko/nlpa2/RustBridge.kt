package ee.nekoko.nlpa2

object RustBridge {
    init {
        System.loadLibrary("rust_lib_nlpa2")
    }

    @JvmStatic
    external fun nativeInit(
        host: NativeHostBridge,
        filesDir: String,
        cacheDir: String,
        packageName: String,
        appVersion: String,
        versionCode: Int,
        isPrivileged: Boolean,
        systemLocale: String,
        entitlementJson: String,
        referenceSizesJson: String,
    ): String

    @JvmStatic
    external fun nativeDispatch(actionJson: String): String

    @JvmStatic
    external fun nativeRender(): String

    @JvmStatic
    external fun nativeOnPlatformEvent(eventJson: String): String
}
