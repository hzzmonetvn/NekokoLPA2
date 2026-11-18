package ee.nekoko.nlpa2

import im.nfc.ccid.CcidPlugin
import org.json.JSONObject

internal interface UiHostHandler {
    fun handleUiOp(op: String, args: JSONObject): HostCallResult
}

internal class NativeHostBridgeImpl(
    private val omapiPlugin: OmapiPlugin,
    private val telephonyPlugin: TelephonyPlugin,
    private val nbridgePlugin: NBridgePlugin,
    private val adbBridgePlugin: AdbBridgePlugin,
    private val qrtrPlugin: QrtrPlugin,
    private val certificatePlugin: CertificatePlugin,
    private val ccidPlugin: CcidPlugin,
    private val httpClient: HttpHostClient,
    private val uiHandler: UiHostHandler,
    private val onStateUpdatedCallback: () -> Unit,
) : NativeHostBridge {
    override fun callHost(requestJson: String): String {
        val result =
            try {
                val request = JSONObject(requestJson)
                val op = request.optString("op", "")
                val args = request.optJSONObject("args") ?: JSONObject()
                dispatchOp(op, args)
            } catch (t: Throwable) {
                val root = t.rootCause()
                HostCallResult.Error("HOST_ERROR", root.readableMessage())
            }
        return result.toJsonString()
    }

    override fun onStateUpdated() {
        onStateUpdatedCallback()
    }

    private fun dispatchOp(op: String, args: JSONObject): HostCallResult {
        val dot = op.indexOf('.')
        if (dot <= 0 || dot >= op.length - 1) {
            return HostCallResult.Error("UNKNOWN_OP", "Unsupported host op: $op")
        }
        val domain = op.substring(0, dot)
        val method = op.substring(dot + 1)

        return when (domain) {
            "omapi" -> omapiPlugin.call(method, args)
            "telephony" -> telephonyPlugin.call(method, args)
            "nbridge" -> nbridgePlugin.call(method, args)
            "adb" -> adbBridgePlugin.call(method, args)
            "qrtr", "seHal" -> qrtrPlugin.call(op, args)
            "ccid" -> ccidPlugin.call(method, args)
            "cert" ->
                if (method == "getInfo") {
                    certificatePlugin.getInfo()
                } else {
                    HostCallResult.Error("UNKNOWN_OP", "Unsupported host op: $op")
                }
            "http" ->
                if (method == "request") {
                    httpClient.request(args)
                } else {
                    HostCallResult.Error("UNKNOWN_OP", "Unsupported host op: $op")
                }
            "ui" -> uiHandler.handleUiOp(op, args)
            else -> HostCallResult.Error("UNKNOWN_OP", "Unsupported host op: $op")
        }
    }
}
