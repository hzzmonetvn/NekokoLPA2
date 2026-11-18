package ee.nekoko.nlpa2

import java.lang.reflect.InvocationTargetException
import org.json.JSONArray
import org.json.JSONObject

internal sealed class HostCallResult {
    data class Success(val data: Any? = true) : HostCallResult()

    data class Error(
        val code: String,
        val message: String,
        val details: Any? = null,
    ) : HostCallResult()

    fun toJsonString(): String {
        val obj = JSONObject()
        when (this) {
            is Success -> {
                obj.put("ok", true)
                obj.put("data", toJsonValue(data))
            }
            is Error -> {
                obj.put("ok", false)
                obj.put("code", code)
                obj.put("message", message)
                if (details != null) {
                    obj.put("details", toJsonValue(details))
                }
            }
        }
        return obj.toString()
    }
}

internal fun interface HostEventSink {
    fun emit(type: String, payload: Map<String, Any?>)
}

internal fun HostEventSink.emit(type: String) = emit(type, emptyMap())

internal fun toJsonValue(value: Any?): Any =
    when (value) {
        null -> JSONObject.NULL
        is JSONObject, is JSONArray -> value
        is Boolean, is Int, is Long, is Double, is Float, is String -> value
        is Short, is Byte -> (value as Number).toInt()
        is Map<*, *> ->
            JSONObject().apply {
                for ((k, v) in value) {
                    if (k != null) {
                        put(k.toString(), toJsonValue(v))
                    }
                }
            }
        is Iterable<*> ->
            JSONArray().apply {
                for (item in value) {
                    put(toJsonValue(item))
                }
            }
        is Array<*> ->
            JSONArray().apply {
                for (item in value) {
                    put(toJsonValue(item))
                }
            }
        is IntArray ->
            JSONArray().apply {
                for (item in value) {
                    put(item)
                }
            }
        is ByteArray -> value.toHexStringLower()
        else -> value.toString()
    }

internal fun ByteArray.toHexStringLower(): String =
    joinToString(separator = "") { "%02x".format(it) }

internal fun String.decodeHexBytes(): ByteArray {
    val clean = replace(" ", "").replace("\n", "").replace("\r", "").trim()
    require(clean.length % 2 == 0) { "Hex string must have an even length" }
    return ByteArray(clean.length / 2) { i ->
        val idx = i * 2
        clean.substring(idx, idx + 2).toInt(16).toByte()
    }
}

internal fun JSONObject.optStringList(key: String): List<String> {
    val array = optJSONArray(key) ?: return emptyList()
    return List(array.length()) { index -> array.optString(index) }.filter { it.isNotEmpty() }
}

internal fun JSONObject.jsonObjectToMap(): Map<String, Any?> {
    val map = linkedMapOf<String, Any?>()
    for (key in keys()) {
        map[key] = normalizeJsonValue(get(key))
    }
    return map
}

private fun normalizeJsonValue(value: Any?): Any? =
    when (value) {
        is JSONObject -> value.jsonObjectToMap()
        is JSONArray -> (0 until value.length()).map { normalizeJsonValue(value.get(it)) }
        JSONObject.NULL -> null
        else -> value
    }

internal fun Throwable.rootCause(): Throwable =
    if (this is InvocationTargetException && targetException != null) {
        targetException.rootCause()
    } else {
        cause?.takeIf { it !== this }?.rootCause() ?: this
    }

internal fun Throwable.readableMessage(): String {
    val msg = message
    return if (msg.isNullOrBlank()) this::class.java.name else "${this::class.java.name}: $msg"
}
