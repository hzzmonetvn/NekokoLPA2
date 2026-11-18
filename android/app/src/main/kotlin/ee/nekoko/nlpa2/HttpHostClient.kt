package ee.nekoko.nlpa2

import android.util.Base64
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.InetSocketAddress
import java.net.Proxy
import java.net.SocketTimeoutException
import java.net.URI
import java.net.URL
import java.security.SecureRandom
import java.security.cert.X509Certificate
import javax.net.ssl.HostnameVerifier
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager
import org.json.JSONObject

internal class HttpHostClient {
    fun request(args: JSONObject): HostCallResult {
        val urlString = args.optString("url", "").trim()
        if (urlString.isEmpty()) {
            return HostCallResult.Error("INVALID_ARGUMENT", "url is required")
        }
        val method = args.optString("method", "GET").uppercase().ifBlank { "GET" }
        val timeoutMs = args.optInt("timeoutMs", DEFAULT_TIMEOUT_MS).coerceIn(1_000, 300_000)
        val proxyUrl = args.optString("proxyUrl", "").trim()
        val bodyBase64 = args.optString("bodyBase64", "")
        val headersObj = args.optJSONObject("headers")
        val explicitInsecure =
            args.optBoolean("insecureTls", false) || args.optBoolean("trustAll", false)
        val followRedirects = args.optBoolean("followRedirects", true)

        var connection: HttpURLConnection? = null
        return try {
            val url = URL(urlString)
            val proxy = parseProxy(proxyUrl)
            val conn =
                (if (proxy != null) url.openConnection(proxy) else url.openConnection())
                    as HttpURLConnection
            connection = conn

            if (conn is HttpsURLConnection && (explicitInsecure || isGsmaRspRequest(urlString, headersObj))) {
                applyScopedGsmaTls(conn)
            }

            conn.requestMethod = method
            conn.connectTimeout = minOf(timeoutMs, DEFAULT_CONNECT_TIMEOUT_MS)
            conn.readTimeout = timeoutMs
            conn.instanceFollowRedirects = followRedirects
            conn.useCaches = false

            if (headersObj != null) {
                for (key in headersObj.keys()) {
                    val value = headersObj.optString(key, "")
                    if (key.isNotEmpty()) {
                        conn.setRequestProperty(key, value)
                    }
                }
            }

            if (bodyBase64.isNotEmpty()) {
                val bodyBytes = Base64.decode(bodyBase64, Base64.DEFAULT)
                conn.doOutput = true
                conn.setFixedLengthStreamingMode(bodyBytes.size)
                conn.outputStream.use { it.write(bodyBytes) }
            } else if (method == "POST" || method == "PUT" || method == "PATCH") {
                conn.doOutput = true
                conn.setFixedLengthStreamingMode(0)
            }

            val status = conn.responseCode
            val stream =
                if (status >= 400) {
                    conn.errorStream
                } else {
                    try {
                        conn.inputStream
                    } catch (_: IOException) {
                        conn.errorStream
                    }
                }
            val respBytes =
                stream?.use { input ->
                    val buffer = ByteArrayOutputStream()
                    val chunk = ByteArray(8192)
                    var total = 0
                    while (true) {
                        val read = input.read(chunk)
                        if (read <= 0) break
                        total += read
                        if (total > MAX_RESPONSE_BYTES) {
                            throw IOException("HTTP response exceeded $MAX_RESPONSE_BYTES bytes")
                        }
                        buffer.write(chunk, 0, read)
                    }
                    buffer.toByteArray()
                } ?: ByteArray(0)

            val respHeaders = linkedMapOf<String, String>()
            val setCookies = mutableListOf<String>()
            for ((key, values) in conn.headerFields) {
                if (key == null || values == null) continue
                if (key.equals("Set-Cookie", ignoreCase = true)) {
                    setCookies.addAll(values)
                }
                respHeaders[key.lowercase()] = values.joinToString(", ")
            }

            HostCallResult.Success(
                mapOf(
                    "status" to status,
                    "headers" to respHeaders,
                    "setCookies" to setCookies,
                    "bodyBase64" to Base64.encodeToString(respBytes, Base64.NO_WRAP),
                ),
            )
        } catch (e: SocketTimeoutException) {
            HostCallResult.Error("HTTP_TIMEOUT", e.message ?: "HTTP request timed out")
        } catch (e: Exception) {
            HostCallResult.Error("HTTP_ERROR", e.message ?: e::class.java.simpleName)
        } finally {
            connection?.disconnect()
        }
    }

    private fun isGsmaRspRequest(url: String, headers: JSONObject?): Boolean {
        if (url.contains("/gsma/rsp", ignoreCase = true)) {
            return true
        }
        if (headers != null) {
            for (key in headers.keys()) {
                if (key.equals("X-Admin-Protocol", ignoreCase = true)) {
                    val value = headers.optString(key, "")
                    if (value.startsWith("gsma/rsp", ignoreCase = true)) {
                        return true
                    }
                }
            }
        }
        return false
    }

    /**
     * SM-DP+ and SM-DS servers present certificates signed by GSMA CI roots that
     * Android's trust store does not carry; the eUICC authenticates the server via
     * GSMA SGP.22 signatures. Apply this trust manager to this connection only.
     */
    private fun applyScopedGsmaTls(conn: HttpsURLConnection) {
        val trustAllCerts =
            arrayOf<TrustManager>(
                object : X509TrustManager {
                    override fun checkClientTrusted(
                        chain: Array<out X509Certificate>?,
                        authType: String?,
                    ) {}

                    override fun checkServerTrusted(
                        chain: Array<out X509Certificate>?,
                        authType: String?,
                    ) {}

                    override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
                },
            )
        val sslContext = SSLContext.getInstance("TLS")
        sslContext.init(null, trustAllCerts, SecureRandom())
        conn.sslSocketFactory = sslContext.socketFactory
        conn.hostnameVerifier = HostnameVerifier { _, _ -> true }
    }

    internal fun parseProxy(proxyUrl: String): Proxy? {
        if (proxyUrl.isBlank()) return null
        if (proxyUrl.equals("DIRECT", ignoreCase = true)) return Proxy.NO_PROXY

        val normalized = if (proxyUrl.contains("://")) proxyUrl else "http://$proxyUrl"
        val uri = URI.create(normalized)
        val host = uri.host ?: throw IllegalArgumentException("Invalid proxy URL: $proxyUrl")
        val isSocks = uri.scheme?.startsWith("socks", ignoreCase = true) == true
        val port =
            if (uri.port > 0) {
                uri.port
            } else if (isSocks) {
                1080
            } else {
                8080
            }
        val type = if (isSocks) Proxy.Type.SOCKS else Proxy.Type.HTTP
        return Proxy(type, InetSocketAddress(host, port))
    }

    companion object {
        private const val DEFAULT_TIMEOUT_MS = 90_000
        private const val DEFAULT_CONNECT_TIMEOUT_MS = 30_000
        private const val MAX_RESPONSE_BYTES = 32 * 1024 * 1024
    }
}
