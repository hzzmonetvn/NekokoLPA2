package ee.nekoko.nlpa2

interface NativeHostBridge {
    fun callHost(requestJson: String): String

    fun onStateUpdated()
}
