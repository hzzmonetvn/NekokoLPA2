package ee.nekoko.nlpa2

import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.HandlerThread
import android.os.Parcelable
import java.io.File
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import org.json.JSONObject

internal class NBridgePlugin {
    private var context: Context? = null
    private var backgroundThread: HandlerThread? = null
    private var backgroundHandler: Handler? = null

    companion object {
        private const val DEFAULT_AUTHORITY = "ee.nekoko.nbridge.provider"
        private const val CALL_TIMEOUT_SECONDS = 60L
    }

    fun attach(appContext: Context) {
        context = appContext.applicationContext
        if (backgroundThread == null) {
            val thread = HandlerThread("NBridgePlugin").apply { start() }
            backgroundThread = thread
            backgroundHandler = Handler(thread.looper)
        }
    }

    fun detach() {
        backgroundThread?.quitSafely()
        backgroundThread = null
        backgroundHandler = null
        context = null
    }

    fun call(method: String, args: JSONObject): HostCallResult {
        if (method == "isAvailable") {
            val authority = args.optString("authority").ifBlank { null }
            return HostCallResult.Success(isProviderAvailable(authority))
        }
        if (method == "isRooted") {
            return HostCallResult.Success(isRooted())
        }

        val handler =
            backgroundHandler
                ?: return HostCallResult.Error("ERROR", "Background handler unavailable")

        val future = CompletableFuture<HostCallResult>()
        val posted =
            handler.post {
                try {
                    val response =
                        when (method) {
                            "listSlots" -> callProvider("listSlots", null)
                            "connectLogicalChannel" -> {
                                val slotId =
                                    args.optString("slotId").ifBlank {
                                        throw IllegalArgumentException("slotId is missing")
                                    }
                                val aids = args.optStringList("aids")
                                callProvider(
                                    method = "connectLogicalChannel",
                                    extras =
                                        Bundle().apply {
                                            putString("slotId", slotId)
                                            putStringArrayList("aids", ArrayList(aids))
                                        },
                                )
                            }
                            "transmitLogical" -> {
                                val connectionId =
                                    args.optString("connectionId").ifBlank {
                                        throw IllegalArgumentException("connectionId is missing")
                                    }
                                val apdu =
                                    args.optString("apdu").ifBlank {
                                        throw IllegalArgumentException("apdu is missing")
                                    }
                                callProvider(
                                    method = "transmitLogical",
                                    extras =
                                        Bundle().apply {
                                            putString("connectionId", connectionId)
                                            putString("apdu", apdu)
                                        },
                                )
                            }
                            "transmitBasic" -> {
                                val slotId =
                                    args.optString("slotId").ifBlank {
                                        throw IllegalArgumentException("slotId is missing")
                                    }
                                val apdu =
                                    args.optString("apdu").ifBlank {
                                        throw IllegalArgumentException("apdu is missing")
                                    }
                                callProvider(
                                    method = "transmitBasic",
                                    extras =
                                        Bundle().apply {
                                            putString("slotId", slotId)
                                            putString("apdu", apdu)
                                        },
                                )
                            }
                            "closeLogicalChannel" -> {
                                val connectionId =
                                    args.optString("connectionId").ifBlank {
                                        throw IllegalArgumentException("connectionId is missing")
                                    }
                                callProvider(
                                    method = "closeLogicalChannel",
                                    extras =
                                        Bundle().apply {
                                            putString("connectionId", connectionId)
                                        },
                                )
                            }
                            else -> throw IllegalArgumentException("Unknown method $method")
                        }

                    val ok = response["ok"] as? Boolean ?: false
                    if (!ok) {
                        val errorCode = (response["errorCode"] as? String) ?: "ERROR"
                        val errorMessage =
                            (response["errorMessage"] as? String) ?: "Unknown NBridge error"
                        future.complete(HostCallResult.Error(errorCode, errorMessage, response))
                    } else {
                        val data: Any? =
                            when (method) {
                                "listSlots" -> response["slots"] ?: emptyList<Any>()
                                "transmitLogical", "transmitBasic" ->
                                    response["responseApdu"]?.toString() ?: ""
                                "closeLogicalChannel" -> true
                                else -> response
                            }
                        future.complete(HostCallResult.Success(data))
                    }
                } catch (t: Throwable) {
                    val root = t.rootCause()
                    future.complete(HostCallResult.Error("ERROR", root.readableMessage()))
                }
            }
        if (!posted) {
            return HostCallResult.Error("ERROR", "Background handler unavailable")
        }

        return try {
            future.get(CALL_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (e: Exception) {
            HostCallResult.Error("ERROR", e.message ?: "NBridge call timed out")
        }
    }

    private fun isProviderAvailable(authority: String?): Boolean {
        val appContext = context ?: return false
        return appContext.packageManager.resolveContentProvider(
            authority ?: DEFAULT_AUTHORITY,
            0,
        ) != null
    }

    private fun isRooted(): Boolean {
        if (Build.TAGS?.contains("test-keys") == true) {
            return true
        }

        val suspiciousPaths =
            listOf(
                "/system/bin/su",
                "/system/xbin/su",
                "/sbin/su",
                "/system_ext/bin/su",
                "/system/bin/.ext/su",
                "/system/usr/we-need-root/su",
                "/vendor/bin/su",
                "/data/adb/magisk",
                "/data/adb/ksu",
                "/data/adb/modules",
                "/cache/magisk.log",
            )
        if (suspiciousPaths.any { path -> File(path).exists() }) {
            return true
        }

        val pathEnv = System.getenv("PATH").orEmpty()
        if (pathEnv.isNotEmpty()) {
            val hasSu =
                pathEnv
                    .split(':')
                    .map { it.trim() }
                    .filter { it.isNotEmpty() }
                    .any { dir -> File(dir, "su").exists() }
            if (hasSu) {
                return true
            }
        }

        return false
    }

    private fun callProvider(method: String, extras: Bundle?): Map<String, Any?> {
        val appContext = context ?: throw IllegalStateException("Context unavailable")
        val authority = DEFAULT_AUTHORITY
        if (!isProviderAvailable(authority)) {
            throw IllegalStateException("NBridge provider is not installed")
        }

        val response =
            appContext.contentResolver.call(
                Uri.parse("content://$authority"),
                method,
                null,
                extras,
            ) ?: Bundle()
        return bundleToMap(response)
    }

    @Suppress("DEPRECATION")
    private fun bundleToMap(bundle: Bundle): Map<String, Any?> {
        val map = linkedMapOf<String, Any?>()
        for (key in bundle.keySet()) {
            map[key] = normalizeValue(bundle.get(key))
        }
        return map
    }

    private fun normalizeValue(value: Any?): Any? =
        when (value) {
            is Bundle -> bundleToMap(value)
            is ArrayList<*> -> value.map { normalizeValue(it) }
            is List<*> -> value.map { normalizeValue(it) }
            is Parcelable -> if (value is Bundle) bundleToMap(value) else value.toString()
            else -> value
        }
}
