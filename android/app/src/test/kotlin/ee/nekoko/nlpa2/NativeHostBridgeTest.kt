package ee.nekoko.nlpa2

import java.net.InetSocketAddress
import java.net.Proxy
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NativeHostBridgeTest {
    @Test
    fun hostCallResultSerializesSuccessWithNullData() {
        val json = JSONObject(HostCallResult.Success(null).toJsonString())
        assertTrue(json.getBoolean("ok"))
        assertTrue(json.isNull("data"))
    }

    @Test
    fun hostCallResultSerializesNestedMapsAndLists() {
        val payload =
            mapOf(
                "sha256" to listOf("AA", "BB"),
                "count" to 2,
                "active" to true,
            )
        val json = JSONObject(HostCallResult.Success(payload).toJsonString())
        assertTrue(json.getBoolean("ok"))
        val data = json.getJSONObject("data")
        assertEquals(2, data.getInt("count"))
        assertTrue(data.getBoolean("active"))
        assertEquals("AA", data.getJSONArray("sha256").getString(0))
    }

    @Test
    fun hostCallResultPreservesOmapiCorruptionErrorAndDetails() {
        val json =
            JSONObject(
                HostCallResult.Error(
                        OMAPI_SESSION_CORRUPTED,
                        "Reboot required",
                        mapOf("rebootRequired" to true, "operationMayHaveSucceeded" to true),
                    )
                    .toJsonString(),
            )
        assertFalse(json.getBoolean("ok"))
        assertEquals(OMAPI_SESSION_CORRUPTED, json.getString("code"))
        assertEquals("Reboot required", json.getString("message"))
        assertTrue(json.getJSONObject("details").getBoolean("rebootRequired"))
    }

    @Test
    fun hexHelpersRoundTrip() {
        val bytes = byteArrayOf(0x00, 0x7F, 0x80.toByte(), 0xFF.toByte())
        val hex = bytes.toHexStringLower()
        assertEquals("007f80ff", hex)
        assertEquals(bytes.toList(), "00 7F\n80 ff".decodeHexBytes().toList())
    }

    @Test
    fun httpClientParsesProxyUrls() {
        val client = HttpHostClient()
        assertNull(client.parseProxy(""))
        assertEquals(Proxy.NO_PROXY, client.parseProxy("DIRECT"))

        val httpProxy = client.parseProxy("http://127.0.0.1:8888")!!
        assertEquals(Proxy.Type.HTTP, httpProxy.type())
        assertEquals(InetSocketAddress("127.0.0.1", 8888), httpProxy.address())

        val socksProxy = client.parseProxy("socks5://10.0.0.2:1080")!!
        assertEquals(Proxy.Type.SOCKS, socksProxy.type())
        assertEquals(InetSocketAddress("10.0.0.2", 1080), socksProxy.address())
    }
}
