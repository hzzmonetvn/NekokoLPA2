package ee.nekoko.nlpa2

import android.annotation.SuppressLint
import android.app.Activity
import android.app.AlertDialog
import android.app.Dialog
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import im.nfc.ccid.CcidPlugin
import java.io.File
import java.io.FileOutputStream
import java.util.Locale
import java.util.concurrent.CompletableFuture
import java.util.concurrent.CountDownLatch
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import org.json.JSONObject

class MainActivity : Activity(), UiHostHandler {
    private lateinit var rootLayout: FrameLayout
    private lateinit var wallpaperView: ImageView
    private lateinit var webView: WebView

    private val omapiPlugin = OmapiPlugin()
    private val telephonyPlugin = TelephonyPlugin()
    private val nbridgePlugin = NBridgePlugin()
    private val adbBridgePlugin = AdbBridgePlugin()
    private val qrtrPlugin = QrtrPlugin()
    private val certificatePlugin = CertificatePlugin()
    private val ccidPlugin = CcidPlugin()
    private val httpClient = HttpHostClient()

    private lateinit var hostBridge: NativeHostBridgeImpl

    private val dispatchExecutor: ExecutorService = Executors.newSingleThreadExecutor()
    private val eventExecutor: ExecutorService = Executors.newSingleThreadExecutor()
    private val renderExecutor: ExecutorService = Executors.newSingleThreadExecutor()
    private val initLatch = CountDownLatch(1)

    @Volatile private var webPageLoaded = false
    @Volatile private var pendingRenderJson: String? = null
    @Volatile private var pendingExportJson: String? = null
    @Volatile private var currentWallpaperSignature: String? = null
    @Volatile private var activeWebsheetDialog: Dialog? = null

    private val eventSink = HostEventSink { type, payload ->
        emitPlatformEvent(type, payload)
    }

    inner class AndroidJsHost {
        @JavascriptInterface
        fun dispatch(actionJson: String) {
            dispatchExecutor.execute {
                try {
                    initLatch.await()
                    val renderJson = RustBridge.nativeDispatch(actionJson)
                    runOnUiThread { applyRenderJson(renderJson) }
                } catch (t: Throwable) {
                    Log.e(TAG, "dispatch failed", t)
                }
            }
        }
    }

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        rootLayout = FrameLayout(this).apply { setBackgroundColor(Color.BLACK) }
        wallpaperView =
            ImageView(this).apply {
                scaleType = ImageView.ScaleType.CENTER_CROP
                visibility = View.GONE
            }
        webView =
            WebView(this).apply {
                setBackgroundColor(Color.BLACK)
                settings.apply {
                    javaScriptEnabled = true
                    domStorageEnabled = true
                    allowFileAccess = false
                    allowContentAccess = false
                    setSupportZoom(false)
                    builtInZoomControls = false
                    displayZoomControls = false
                    mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                }
                val jsHost = AndroidJsHost()
                addJavascriptInterface(jsHost, "AndroidHost")
                addJavascriptInterface(jsHost, "AndroidJsHost")
                webViewClient =
                    object : WebViewClient() {
                        override fun onPageFinished(view: WebView?, url: String?) {
                            webPageLoaded = true
                            val pending = pendingRenderJson
                            pendingRenderJson = null
                            if (pending != null) {
                                applyRenderJson(pending)
                            }
                        }

                        override fun shouldOverrideUrlLoading(
                            view: WebView?,
                            request: WebResourceRequest?,
                        ): Boolean {
                            val target = request?.url ?: return false
                            val targetStr = target.toString()
                            if (targetStr.startsWith(BASE_URL)) {
                                return false
                            }
                            if (target.scheme == "http" || target.scheme == "https") {
                                openExternalUrl(targetStr)
                                return true
                            }
                            return true
                        }
                    }
            }

        rootLayout.addView(
            wallpaperView,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        rootLayout.addView(
            webView,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        setContentView(rootLayout)
        configureEdgeToEdge()

        hostBridge =
            NativeHostBridgeImpl(
                omapiPlugin = omapiPlugin,
                telephonyPlugin = telephonyPlugin,
                nbridgePlugin = nbridgePlugin,
                adbBridgePlugin = adbBridgePlugin,
                qrtrPlugin = qrtrPlugin,
                certificatePlugin = certificatePlugin,
                ccidPlugin = ccidPlugin,
                httpClient = httpClient,
                uiHandler = this,
                onStateUpdatedCallback = { scheduleAsyncRender() },
            )

        val appCtx = applicationContext
        omapiPlugin.attach(appCtx, eventSink)
        telephonyPlugin.attach(appCtx, eventSink)
        nbridgePlugin.attach(appCtx)
        adbBridgePlugin.attach(appCtx, eventSink)
        qrtrPlugin.attach(appCtx)
        certificatePlugin.attach(appCtx)
        ccidPlugin.attach(appCtx, eventSink)

        val launchDeepLink = extractDeepLinkUrl(intent)

        dispatchExecutor.execute {
            try {
                val entitlementJson =
                    readAssetText("ts43/entitlement.json", "assets/ts43/entitlement.json") ?: "[]"
                val referenceSizesJson =
                    readAssetText("reference_sizes_simple.json", "data/reference_sizes_simple.json")
                        ?: "{}"

                val initResult =
                    RustBridge.nativeInit(
                        host = hostBridge,
                        filesDir = filesDir.absolutePath,
                        cacheDir = cacheDir.absolutePath,
                        packageName = packageName,
                        appVersion = BuildConfig.VERSION_NAME,
                        versionCode = BuildConfig.VERSION_CODE,
                        isPrivileged = BuildConfig.PRIVILEGED_BUILD,
                        systemLocale = Locale.getDefault().toLanguageTag(),
                        entitlementJson = entitlementJson,
                        referenceSizesJson = referenceSizesJson,
                    )

                val initialRenderJson =
                    if (initResult.trimStart().startsWith("{")) {
                        initResult
                    } else {
                        runCatching { RustBridge.nativeRender() }.getOrNull()
                    }
                val initialHtml =
                    if (initResult.trimStart().startsWith("{")) {
                        runCatching { JSONObject(initResult).optString("html", "") }.getOrDefault("")
                    } else {
                        initResult
                    }

                runOnUiThread {
                    if (initialRenderJson != null) {
                        applyVisualStateFromRenderJson(initialRenderJson)
                    }
                    webView.loadDataWithBaseURL(
                        BASE_URL,
                        initialHtml,
                        "text/html",
                        "UTF-8",
                        null,
                    )
                }
            } catch (t: Throwable) {
                Log.e(TAG, "nativeInit failed", t)
            } finally {
                initLatch.countDown()
            }

            if (launchDeepLink != null) {
                emitPlatformEvent("deep_link", mapOf("url" to launchDeepLink))
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        val url = extractDeepLinkUrl(intent)
        if (url != null) {
            emitPlatformEvent("deep_link", mapOf("url" to url))
        }
    }

    override fun onDestroy() {
        activeWebsheetDialog?.dismiss()
        activeWebsheetDialog = null

        omapiPlugin.detach()
        telephonyPlugin.detach()
        nbridgePlugin.detach()
        adbBridgePlugin.detach()
        qrtrPlugin.detach()
        certificatePlugin.detach()
        ccidPlugin.detach()

        dispatchExecutor.shutdown()
        eventExecutor.shutdown()
        renderExecutor.shutdown()

        webView.stopLoading()
        webView.destroy()
        super.onDestroy()
    }

    private fun extractDeepLinkUrl(intent: Intent?): String? {
        if (intent?.action != Intent.ACTION_VIEW) return null
        val uri = intent.data ?: return null
        val scheme = uri.scheme?.lowercase() ?: return null
        val host = uri.host?.lowercase() ?: ""
        return when {
            scheme == "lpa" -> uri.toString()
            (scheme == "https" || scheme == "http") &&
                (host == "install.lpa.ee" || host == "esimsetup.lpa.ee") -> uri.toString()
            else -> null
        }
    }

    private fun scheduleAsyncRender() {
        if (renderExecutor.isShutdown) return
        renderExecutor.execute {
            try {
                initLatch.await()
                val renderJson = RustBridge.nativeRender()
                runOnUiThread { applyRenderJson(renderJson) }
            } catch (t: Throwable) {
                Log.w(TAG, "async nativeRender failed", t)
            }
        }
    }

    private fun emitPlatformEvent(type: String, payload: Map<String, Any?>) {
        if (eventExecutor.isShutdown) return
        val json =
            JSONObject().apply {
                put("type", type)
                for ((k, v) in payload) {
                    put(k, toJsonValue(v))
                }
            }.toString()
        eventExecutor.execute {
            try {
                initLatch.await()
                val renderJson = RustBridge.nativeOnPlatformEvent(json)
                if (renderJson.isNotBlank()) {
                    runOnUiThread { applyRenderJson(renderJson) }
                }
            } catch (t: Throwable) {
                Log.w(TAG, "nativeOnPlatformEvent($type) failed", t)
            }
        }
    }

    private fun applyRenderJson(renderJson: String) {
        if (renderJson.isBlank()) return
        val parsed = runCatching { JSONObject(renderJson) }.getOrNull() ?: return
        applyVisualState(parsed)
        if (!webPageLoaded) {
            pendingRenderJson = renderJson
            return
        }
        val html = parsed.optString("html", "")
        if (html.isNotEmpty()) {
            val quoted =
                JSONObject.quote(html)
                    .replace("\u2028", "\\u2028")
                    .replace("\u2029", "\\u2029")
            webView.evaluateJavascript(
                "window.__nlpaPatch && window.__nlpaPatch($quoted);",
                null,
            )
        }
    }

    private fun applyVisualStateFromRenderJson(renderJson: String) {
        val parsed = runCatching { JSONObject(renderJson) }.getOrNull() ?: return
        applyVisualState(parsed)
    }

    private fun applyVisualState(parsed: JSONObject) {
        val themeStyle = parsed.optString("themeStyle", "amoled")
        val dark = parsed.optBoolean("dark", true)
        val wallpaperActive = parsed.optBoolean("wallpaperActive", false)
        val wallpaperOpacity = parsed.optDouble("wallpaperOpacity", 0.35).toFloat().coerceIn(0.05f, 1.0f)
        val statusBarBlack = parsed.optBoolean("statusBarBlack", themeStyle == "amoled" && dark)

        val baseColor =
            when {
                dark && (statusBarBlack || themeStyle == "amoled") -> Color.BLACK
                dark -> Color.parseColor("#121212")
                else -> Color.parseColor("#FAFAFA")
            }
        rootLayout.setBackgroundColor(baseColor)

        val wallpaperFile = wallpaperFile()
        if (wallpaperActive && wallpaperFile.exists()) {
            ensureWallpaperBitmapLoaded(wallpaperFile)
            wallpaperView.alpha = wallpaperOpacity
            wallpaperView.visibility = View.VISIBLE
            webView.setBackgroundColor(Color.TRANSPARENT)
        } else {
            wallpaperView.visibility = View.GONE
            webView.setBackgroundColor(baseColor)
        }

        updateSystemBarAppearance(
            dark = dark,
            statusBarBlack = statusBarBlack,
            wallpaperActive = wallpaperActive && wallpaperFile.exists(),
            baseColor = baseColor,
        )
    }

    private fun wallpaperFile(): File = File(filesDir, WALLPAPER_FILENAME)

    private fun ensureWallpaperBitmapLoaded(file: File) {
        val signature = "${file.length()}:${file.lastModified()}"
        if (signature == currentWallpaperSignature && wallpaperView.drawable != null) {
            return
        }
        val bitmap = decodeSampledBitmap(file, 2048, 2048)
        if (bitmap != null) {
            wallpaperView.setImageBitmap(bitmap)
            currentWallpaperSignature = signature
        }
    }

    private fun decodeSampledBitmap(file: File, reqWidth: Int, reqHeight: Int): Bitmap? {
        return try {
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeFile(file.absolutePath, bounds)
            var inSampleSize = 1
            if (bounds.outHeight > reqHeight || bounds.outWidth > reqWidth) {
                val halfHeight = bounds.outHeight / 2
                val halfWidth = bounds.outWidth / 2
                while (
                    (halfHeight / inSampleSize) >= reqHeight &&
                        (halfWidth / inSampleSize) >= reqWidth
                ) {
                    inSampleSize *= 2
                }
            }
            val opts = BitmapFactory.Options().apply { this.inSampleSize = inSampleSize }
            BitmapFactory.decodeFile(file.absolutePath, opts)
        } catch (t: Throwable) {
            Log.w(TAG, "Failed to decode wallpaper bitmap", t)
            null
        }
    }

    private fun configureEdgeToEdge() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            window.setDecorFitsSystemWindows(false)
        } else {
            @Suppress("DEPRECATION")
            window.decorView.systemUiVisibility =
                View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
                    View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION or
                    View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
        }

        rootLayout.setOnApplyWindowInsetsListener { _, windowInsets ->
            val lp = webView.layoutParams as FrameLayout.LayoutParams
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val insets =
                    windowInsets.getInsets(
                        WindowInsets.Type.systemBars() or
                            WindowInsets.Type.ime() or
                            WindowInsets.Type.displayCutout(),
                    )
                lp.setMargins(insets.left, insets.top, insets.right, insets.bottom)
            } else {
                @Suppress("DEPRECATION")
                lp.setMargins(
                    windowInsets.systemWindowInsetLeft,
                    windowInsets.systemWindowInsetTop,
                    windowInsets.systemWindowInsetRight,
                    windowInsets.systemWindowInsetBottom,
                )
            }
            webView.layoutParams = lp
            windowInsets
        }
    }

    @Suppress("DEPRECATION")
    private fun updateSystemBarAppearance(
        dark: Boolean,
        statusBarBlack: Boolean,
        wallpaperActive: Boolean,
        baseColor: Int,
    ) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.VANILLA_ICE_CREAM) {
            val barColor =
                when {
                    wallpaperActive -> Color.TRANSPARENT
                    statusBarBlack -> Color.BLACK
                    else -> baseColor
                }
            window.statusBarColor = barColor
            window.navigationBarColor = barColor
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val controller = window.insetsController ?: return
            val mask =
                WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS or
                    WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS
            val appearance = if (dark) 0 else mask
            controller.setSystemBarsAppearance(appearance, mask)
        } else {
            var flags =
                View.SYSTEM_UI_FLAG_LAYOUT_STABLE or
                    View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION or
                    View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
            if (!dark) {
                flags =
                    flags or
                        View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR or
                        View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR
            }
            window.decorView.systemUiVisibility = flags
        }
    }

    override fun handleUiOp(op: String, args: JSONObject): HostCallResult {
        return when (op) {
            "ui.copyClipboard" -> {
                val text = args.optString("text", "")
                runOnMainSync {
                    val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                    clipboard.setPrimaryClip(ClipData.newPlainText("NekokoLPA2", text))
                }
                HostCallResult.Success(true)
            }
            "ui.readClipboard" -> {
                val text =
                    runOnMainSync {
                        val clipboard =
                            getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                        val clip = clipboard.primaryClip
                        if (clip != null && clip.itemCount > 0) {
                            clip.getItemAt(0).coerceToText(this)?.toString() ?: ""
                        } else {
                            ""
                        }
                    }
                HostCallResult.Success(mapOf("text" to text))
            }
            "ui.openUrl" -> {
                val url = args.optString("url", "").trim()
                if (url.isEmpty()) {
                    return HostCallResult.Error("INVALID_ARGUMENT", "url is required")
                }
                val ok = openExternalUrl(url)
                if (ok) {
                    HostCallResult.Success(true)
                } else {
                    HostCallResult.Error("OPEN_URL_FAILED", "No app found to handle URL")
                }
            }
            "ui.pickWallpaper" -> {
                runOnUiThread {
                    val intent =
                        Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                            addCategory(Intent.CATEGORY_OPENABLE)
                            type = "image/*"
                        }
                    @Suppress("DEPRECATION")
                    startActivityForResult(intent, REQ_PICK_WALLPAPER)
                }
                HostCallResult.Success(true)
            }
            "ui.clearWallpaper" -> {
                val file = wallpaperFile()
                file.delete()
                File(filesDir, "$WALLPAPER_FILENAME.tmp").delete()
                currentWallpaperSignature = null
                runOnUiThread {
                    wallpaperView.setImageDrawable(null)
                    wallpaperView.visibility = View.GONE
                }
                emitPlatformEvent("wallpaper_changed", mapOf("active" to false))
                HostCallResult.Success(true)
            }
            "ui.exportState" -> {
                val json = args.optString("json", "")
                if (json.isEmpty()) {
                    return HostCallResult.Error("INVALID_ARGUMENT", "json is required")
                }
                pendingExportJson = json
                runOnUiThread {
                    val intent =
                        Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                            addCategory(Intent.CATEGORY_OPENABLE)
                            type = "application/json"
                            putExtra(Intent.EXTRA_TITLE, "nekokolpa2-backup.json")
                        }
                    @Suppress("DEPRECATION")
                    startActivityForResult(intent, REQ_EXPORT_STATE)
                }
                HostCallResult.Success(true)
            }
            "ui.importState" -> {
                runOnUiThread {
                    val intent =
                        Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                            addCategory(Intent.CATEGORY_OPENABLE)
                            type = "*/*"
                        }
                    @Suppress("DEPRECATION")
                    startActivityForResult(intent, REQ_IMPORT_STATE)
                }
                HostCallResult.Success(true)
            }
            "ui.openWebsheet" -> {
                val url = args.optString("url", "").trim()
                val postData = args.optString("postData", "")
                val uri = Uri.parse(url)
                if (uri.scheme != "https" && uri.scheme != "http") {
                    return HostCallResult.Error(
                        "INVALID_ARGUMENT",
                        "Websheet URL must use http or https",
                    )
                }
                runOnUiThread { showWebsheetDialog(url, postData) }
                HostCallResult.Success(true)
            }
            "ui.scanQr" -> {
                runOnUiThread { launchQrScanOrFallbackDialog() }
                HostCallResult.Success(true)
            }
            else -> HostCallResult.Error("UNKNOWN_OP", "Unsupported UI op: $op")
        }
    }

    private fun openExternalUrl(url: String): Boolean {
        val uri = runCatching { Uri.parse(url) }.getOrNull() ?: return false
        val scheme = uri.scheme?.lowercase() ?: return false
        if (scheme !in setOf("https", "http", "lpa", "mailto")) {
            return false
        }
        return try {
            runOnMainSync {
                startActivity(Intent(Intent.ACTION_VIEW, uri))
            }
            true
        } catch (_: ActivityNotFoundException) {
            false
        } catch (t: Throwable) {
            Log.w(TAG, "Failed to open URL $url", t)
            false
        }
    }

    private fun launchQrScanOrFallbackDialog() {
        val scanIntent = Intent("com.google.zxing.client.android.SCAN").apply {
            putExtra("SCAN_MODE", "QR_CODE_MODE")
        }
        if (scanIntent.resolveActivity(packageManager) != null) {
            try {
                @Suppress("DEPRECATION")
                startActivityForResult(scanIntent, REQ_SCAN_QR)
                return
            } catch (_: Exception) {}
        }
        showQrFallbackDialog()
    }

    private fun showQrFallbackDialog() {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clipText =
            clipboard.primaryClip
                ?.takeIf { it.itemCount > 0 }
                ?.getItemAt(0)
                ?.coerceToText(this)
                ?.toString()
                ?.trim()
                .orEmpty()

        val container =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(48, 24, 48, 8)
            }
        val input =
            EditText(this).apply {
                hint = "LPA:1\$smdp.example.com\$MATCHING_ID"
                if (clipText.startsWith("LPA:", ignoreCase = true)) {
                    setText(clipText)
                    setSelection(clipText.length)
                }
            }
        val pasteBtn =
            Button(this).apply {
                text = "Paste Clipboard"
                setOnClickListener {
                    val freshClip =
                        clipboard.primaryClip
                            ?.takeIf { it.itemCount > 0 }
                            ?.getItemAt(0)
                            ?.coerceToText(this@MainActivity)
                            ?.toString()
                            ?.trim()
                            .orEmpty()
                    if (freshClip.isNotEmpty()) {
                        input.setText(freshClip)
                        input.setSelection(freshClip.length)
                    }
                }
            }
        container.addView(input)
        container.addView(pasteBtn)

        AlertDialog.Builder(this, android.R.style.Theme_DeviceDefault_Dialog_Alert)
            .setTitle("Scan or Enter LPA QR")
            .setView(container)
            .setPositiveButton("Use Code") { _, _ ->
                val code = input.text.toString().trim()
                if (code.isNotEmpty()) {
                    emitPlatformEvent("qr_scanned", mapOf("code" to code))
                } else {
                    emitPlatformEvent("qr_scanned", mapOf("cancelled" to true))
                }
            }
            .setNegativeButton("Cancel") { _, _ ->
                emitPlatformEvent("qr_scanned", mapOf("cancelled" to true))
            }
            .setOnCancelListener {
                emitPlatformEvent("qr_scanned", mapOf("cancelled" to true))
            }
            .show()
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun showWebsheetDialog(url: String, postData: String) {
        activeWebsheetDialog?.dismiss()

        val completed = AtomicBoolean(false)
        val dialog = Dialog(this, android.R.style.Theme_Black_NoTitleBar_Fullscreen)
        activeWebsheetDialog = dialog

        val root =
            LinearLayout(this).apply {
                orientation = LinearLayout.VERTICAL
                setBackgroundColor(Color.BLACK)
            }

        val header =
            LinearLayout(this).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                setPadding(24, 16, 24, 16)
                setBackgroundColor(Color.parseColor("#141414"))
            }
        val title =
            TextView(this).apply {
                text = "Carrier Verification"
                setTextColor(Color.WHITE)
                textSize = 16f
                layoutParams = LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f)
            }
        val closeButton =
            Button(this).apply {
                text = "Close"
                setOnClickListener { dialog.dismiss() }
            }
        header.addView(title)
        header.addView(closeButton)

        val odsaJs =
            readAssetText("ts43/odsa_service_flow.js", "assets/ts43/odsa_service_flow.js")
                .orEmpty()
        val bridgeJs =
            """
            (function(g){
              function post(line){ try { TS43Channel.postMessage(line); } catch(e){} }
              g.addEventListener('ts43-odsa-callback', function(ev){
                try { post('TS43_ODSA_CALLBACK ' + JSON.stringify(ev.detail)); } catch(e){}
              });
              var orig = g.console && g.console.log;
              if (orig) {
                g.console.log = function(){
                  try {
                    var s = Array.prototype.join.call(arguments, ' ');
                    if (s.indexOf('TS43_ODSA_CALLBACK') === 0 || s.indexOf('TS43_VOWIFI_CALLBACK') === 0) {
                      post(s);
                    }
                  } catch(e){}
                  return orig.apply(g.console, arguments);
                };
              }
            })(window);
            """.trimIndent()

        val sheetWebView =
            WebView(this).apply {
                settings.apply {
                    javaScriptEnabled = true
                    domStorageEnabled = true
                    allowFileAccess = false
                    allowContentAccess = false
                }
                addJavascriptInterface(
                    object : Any() {
                        @JavascriptInterface
                        fun postMessage(line: String) {
                            if (completed.compareAndSet(false, true)) {
                                emitPlatformEvent(
                                    "websheet_result",
                                    mapOf("raw" to line, "dismissed" to false),
                                )
                                runOnUiThread { dialog.dismiss() }
                            }
                        }
                    },
                    "TS43Channel",
                )
                webViewClient =
                    object : WebViewClient() {
                        override fun onPageFinished(view: WebView?, loadedUrl: String?) {
                            if (odsaJs.isNotEmpty()) {
                                view?.evaluateJavascript(odsaJs, null)
                            }
                            view?.evaluateJavascript(bridgeJs, null)
                        }

                        override fun shouldOverrideUrlLoading(
                            view: WebView?,
                            request: WebResourceRequest?,
                        ): Boolean {
                            val nextUrl = request?.url?.toString() ?: return false
                            if (nextUrl.contains("/_callback")) {
                                if (completed.compareAndSet(false, true)) {
                                    emitPlatformEvent(
                                        "websheet_result",
                                        mapOf("raw" to nextUrl, "dismissed" to false),
                                    )
                                    dialog.dismiss()
                                }
                                return true
                            }
                            return false
                        }
                    }
            }

        root.addView(
            header,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )
        root.addView(
            sheetWebView,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                0,
                1f,
            ),
        )

        dialog.setContentView(root)
        dialog.setOnDismissListener {
            if (activeWebsheetDialog === dialog) {
                activeWebsheetDialog = null
            }
            if (completed.compareAndSet(false, true)) {
                emitPlatformEvent(
                    "websheet_result",
                    mapOf(
                        "raw" to "TS43_ODSA_CALLBACK {\"event\":\"dismissFlow\"}",
                        "dismissed" to true,
                    ),
                )
            }
            sheetWebView.stopLoading()
            sheetWebView.destroy()
        }

        if (postData.isNotEmpty()) {
            sheetWebView.postUrl(url, postData.toByteArray(Charsets.UTF_8))
        } else {
            sheetWebView.loadUrl(url)
        }
        dialog.show()
    }

    @Deprecated("Deprecated in Android API")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        when (requestCode) {
            REQ_PICK_WALLPAPER -> {
                if (resultCode != RESULT_OK || data?.data == null) return
                val uri = data.data!!
                dispatchExecutor.execute {
                    val saved = saveWallpaperFromUri(uri)
                    if (saved != null) {
                        runOnUiThread {
                            ensureWallpaperBitmapLoaded(saved)
                            wallpaperView.visibility = View.VISIBLE
                            webView.setBackgroundColor(Color.TRANSPARENT)
                        }
                        emitPlatformEvent(
                            "wallpaper_changed",
                            mapOf("active" to true, "path" to saved.absolutePath),
                        )
                    }
                }
            }
            REQ_EXPORT_STATE -> {
                val json = pendingExportJson
                pendingExportJson = null
                if (resultCode != RESULT_OK || data?.data == null || json == null) return
                val uri = data.data!!
                dispatchExecutor.execute {
                    try {
                        contentResolver.openOutputStream(uri)?.use { out ->
                            out.write(json.toByteArray(Charsets.UTF_8))
                        }
                    } catch (t: Throwable) {
                        Log.e(TAG, "Failed to export state", t)
                    }
                }
            }
            REQ_IMPORT_STATE -> {
                if (resultCode != RESULT_OK || data?.data == null) return
                val uri = data.data!!
                dispatchExecutor.execute {
                    try {
                        val content = readBoundedUtf8(uri, MAX_IMPORT_BYTES)
                        if (content != null) {
                            emitPlatformEvent("state_imported", mapOf("json" to content))
                        }
                    } catch (t: Throwable) {
                        Log.e(TAG, "Failed to import state", t)
                    }
                }
            }
            REQ_SCAN_QR -> {
                if (resultCode == RESULT_OK) {
                    val scanned = data?.getStringExtra("SCAN_RESULT")?.trim()
                    if (!scanned.isNullOrEmpty()) {
                        emitPlatformEvent("qr_scanned", mapOf("code" to scanned))
                    } else {
                        showQrFallbackDialog()
                    }
                } else {
                    emitPlatformEvent("qr_scanned", mapOf("cancelled" to true))
                }
            }
        }
    }

    private fun saveWallpaperFromUri(uri: Uri): File? {
        return try {
            val tmpFile = File(filesDir, "$WALLPAPER_FILENAME.tmp")
            val targetFile = wallpaperFile()
            var total = 0L
            contentResolver.openInputStream(uri)?.use { input ->
                FileOutputStream(tmpFile).use { output ->
                    val buffer = ByteArray(8192)
                    while (true) {
                        val read = input.read(buffer)
                        if (read <= 0) break
                        total += read
                        if (total > MAX_WALLPAPER_BYTES) {
                            throw IllegalArgumentException("Wallpaper image too large")
                        }
                        output.write(buffer, 0, read)
                    }
                }
            } ?: return null
            if (total == 0L) {
                tmpFile.delete()
                return null
            }
            if (!tmpFile.renameTo(targetFile)) {
                tmpFile.copyTo(targetFile, overwrite = true)
                tmpFile.delete()
            }
            currentWallpaperSignature = null
            targetFile
        } catch (t: Throwable) {
            Log.e(TAG, "Failed to save wallpaper", t)
            File(filesDir, "$WALLPAPER_FILENAME.tmp").delete()
            null
        }
    }

    private fun readBoundedUtf8(uri: Uri, maxBytes: Int): String? {
        contentResolver.openInputStream(uri)?.use { input ->
            val buffer = ByteArray(8192)
            val bytes = java.io.ByteArrayOutputStream()
            var total = 0
            while (true) {
                val read = input.read(buffer)
                if (read <= 0) break
                total += read
                if (total > maxBytes) {
                    throw IllegalArgumentException("Imported file exceeds $maxBytes bytes")
                }
                bytes.write(buffer, 0, read)
            }
            return bytes.toByteArray().toString(Charsets.UTF_8)
        }
        return null
    }

    private fun readAssetText(vararg candidatePaths: String): String? {
        for (path in candidatePaths) {
            try {
                return assets.open(path).bufferedReader(Charsets.UTF_8).use { it.readText() }
            } catch (_: Exception) {
                // Try fallback path
            }
        }
        return null
    }

    private fun <T> runOnMainSync(block: () -> T): T {
        if (mainLooper.isCurrentThread) {
            return block()
        }
        val future = CompletableFuture<T>()
        runOnUiThread {
            try {
                future.complete(block())
            } catch (t: Throwable) {
                future.completeExceptionally(t)
            }
        }
        return future.get(15, TimeUnit.SECONDS)
    }

    companion object {
        private const val TAG = "MainActivity"
        private const val BASE_URL = "https://appassets.androidplatform.net/"
        private const val WALLPAPER_FILENAME = "wallpaper.img"
        private const val MAX_WALLPAPER_BYTES = 32L * 1024 * 1024
        private const val MAX_IMPORT_BYTES = 16 * 1024 * 1024

        private const val REQ_PICK_WALLPAPER = 4101
        private const val REQ_EXPORT_STATE = 4102
        private const val REQ_IMPORT_STATE = 4103
        private const val REQ_SCAN_QR = 4104
    }
}
