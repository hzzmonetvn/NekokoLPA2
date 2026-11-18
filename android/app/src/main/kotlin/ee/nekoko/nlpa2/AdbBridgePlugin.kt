package ee.nekoko.nlpa2

import android.content.Context
import android.util.Log
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject

/**
 * The app's side of the ADB bridge.
 *
 * It shows the command a person runs over ADB, holds the connection window
 * open while they do, and — once a `shell` helper has registered through
 * [AdbBridgeProvider] — forwards the card operations to it. The operations are
 * the ones [NBridgePlugin] already drives, so the host protocol is shared in
 * shape; only the transport differs.
 *
 * Calls into the helper block on the binder, so they run off the UI thread.
 * State changes are forwarded to the host event sink when a helper comes and goes.
 */
internal class AdbBridgePlugin : AdbBridgeHub.Listener {
    private var context: Context? = null
    private var eventSink: HostEventSink? = null
    private var worker: ExecutorService? = null

    fun attach(appContext: Context, sink: HostEventSink) {
        context = appContext.applicationContext
        eventSink = sink
        if (worker == null || worker?.isShutdown == true) {
            worker = Executors.newSingleThreadExecutor()
        }
        AdbBridgeHub.listener = this
    }

    fun detach() {
        if (AdbBridgeHub.listener === this) {
            AdbBridgeHub.listener = null
        }
        AdbBridgeHub.closeWindow()
        eventSink = null
        worker?.shutdown()
        worker = null
        context = null
    }

    override fun onBridgeConnected() {
        eventSink?.emit("adb_bridge_state", mapOf("connected" to true))
    }

    override fun onBridgeDisconnected() {
        eventSink?.emit("adb_bridge_state", mapOf("connected" to false))
    }

    fun call(method: String, args: JSONObject): HostCallResult {
        return when (method) {
            "getConnectCommand" -> HostCallResult.Success(connectCommand())
            "isConnected" -> HostCallResult.Success(AdbBridgeHub.isConnected())
            "beginConnect" -> {
                val requested = args.optLong("timeoutMs", DEFAULT_WINDOW_MS)
                val timeout = if (requested > 0) requested else DEFAULT_WINDOW_MS
                AdbBridgeHub.openWindow(timeout)
                HostCallResult.Success(timeout)
            }
            "cancelConnect" -> {
                AdbBridgeHub.closeWindow()
                HostCallResult.Success(true)
            }
            "disconnect" -> disconnect()
            "listSlots" -> forward("listSlots") { it.listSlots() }
            "connectLogicalChannel" -> {
                val slotId = args.optString("slotId").ifBlank { invalid("slotId") }
                val aids = args.optStringList("aids")
                forward("connectLogicalChannel") { it.openLogicalChannel(slotId, aids) }
            }
            "transmitLogical" -> {
                val connectionId =
                    args.optString("connectionId").ifBlank { invalid("connectionId") }
                val apdu = args.optString("apdu").ifBlank { invalid("apdu") }
                forward("transmitLogical") { it.transmitLogical(connectionId, apdu) }
            }
            "transmitBasic" -> {
                val slotId = args.optString("slotId").ifBlank { invalid("slotId") }
                val apdu = args.optString("apdu").ifBlank { invalid("apdu") }
                forward("transmitBasic") { it.transmitBasic(slotId, apdu) }
            }
            "closeLogicalChannel" -> {
                val connectionId =
                    args.optString("connectionId").ifBlank { invalid("connectionId") }
                forward("closeLogicalChannel") { it.closeLogicalChannel(connectionId) }
            }
            else -> HostCallResult.Error("UNSUPPORTED_METHOD", "Unknown ADB bridge method: $method")
        }
    }

    private fun invalid(name: String): Nothing = throw IllegalArgumentException("$name is missing")

    /** Build the command for whichever flavour is installed. */
    private fun connectCommand(): String {
        val pkg = context?.packageName ?: BuildConfig.APPLICATION_ID
        return "adb shell \"CLASSPATH=\\\$(pm path $pkg | sed -n 's/^package://p' | " +
            "head -n1) app_process / ee.nekoko.nlpa2.AdbBridgeStarter $pkg\""
    }

    /** Run `call` against the helper off the calling thread, JSON answer to [HostCallResult]. */
    private fun forward(method: String, call: (IAdbBridge) -> String): HostCallResult {
        val binder = AdbBridgeHub.bridge()
        if (binder == null || !binder.isBinderAlive) {
            return HostCallResult.Error("NO_BRIDGE", "No computer connection is active")
        }

        val executor =
            worker ?: return HostCallResult.Error("ERROR", "ADB bridge worker unavailable")
        val service = IAdbBridge.Stub.asInterface(binder)
        val future = CompletableFuture<HostCallResult>()
        executor.execute {
            try {
                val obj = JSONObject(call(service))
                val ok = obj.optBoolean("ok", false)
                if (!ok) {
                    val code = obj.optString("errorCode", "ERROR").ifBlank { "ERROR" }
                    val message =
                        obj.optString("errorMessage", "Unknown ADB bridge error")
                            .ifBlank { "Unknown ADB bridge error" }
                    future.complete(HostCallResult.Error(code, message, obj.jsonObjectToMap()))
                } else {
                    val data: Any? =
                        when (method) {
                            "listSlots" -> obj.optJSONArray("slots") ?: JSONArray()
                            "transmitLogical", "transmitBasic" ->
                                obj.optString("responseApdu", "")
                            "closeLogicalChannel" -> true
                            else -> obj.jsonObjectToMap()
                        }
                    future.complete(HostCallResult.Success(data))
                }
            } catch (e: Throwable) {
                Log.w(TAG, "helper call failed", e)
                future.complete(
                    HostCallResult.Error("BRIDGE_CALL_FAILED", e.message ?: e.toString()),
                )
            }
        }
        return try {
            future.get(CALL_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (e: Exception) {
            HostCallResult.Error("BRIDGE_CALL_FAILED", e.message ?: "ADB bridge timed out")
        }
    }

    private fun disconnect(): HostCallResult {
        val binder = AdbBridgeHub.bridge()
        val executor = worker
        if (executor == null) {
            AdbBridgeHub.disconnect()
            return HostCallResult.Success(true)
        }
        val future = CompletableFuture<HostCallResult>()
        executor.execute {
            if (binder != null && binder.isBinderAlive) {
                try {
                    // The helper exits while answering, so a dead binder here is
                    // the expected outcome, not a failure.
                    IAdbBridge.Stub.asInterface(binder).shutdown()
                } catch (_: Throwable) {}
            }
            AdbBridgeHub.disconnect()
            future.complete(HostCallResult.Success(true))
        }
        return try {
            future.get(CALL_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (_: Exception) {
            AdbBridgeHub.disconnect()
            HostCallResult.Success(true)
        }
    }

    companion object {
        private const val TAG = "AdbBridgePlugin"
        private const val DEFAULT_WINDOW_MS = 120_000L
        private const val CALL_TIMEOUT_SECONDS = 60L
    }
}
