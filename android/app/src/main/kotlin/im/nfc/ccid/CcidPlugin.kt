package im.nfc.ccid

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbConstants
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbEndpoint
import android.hardware.usb.UsbInterface
import android.hardware.usb.UsbManager
import android.os.Build
import android.util.Log
import ee.nekoko.nlpa2.HostCallResult
import ee.nekoko.nlpa2.HostEventSink
import ee.nekoko.nlpa2.decodeHexBytes
import ee.nekoko.nlpa2.toHexStringLower
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.json.JSONObject

/** Pure-Android USB CCID manager refactored from the local ccid plugin without any bridge runtime deps. */
internal class CcidPlugin {
    private var context: Context? = null
    private var usbManager: UsbManager? = null
    private var eventSink: HostEventSink? = null
    private var worker: ExecutorService? = null
    private var receiverRegistered = false

    private val readers = ConcurrentHashMap<String, Reader>()
    private val readerATR = ConcurrentHashMap<Int, String>()

    private val usbReceiver =
        object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                if (intent.action != ACTION_USB_PERMISSION) return

                val device: UsbDevice? =
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                        intent.getParcelableExtra(UsbManager.EXTRA_DEVICE, UsbDevice::class.java)
                    } else {
                        @Suppress("DEPRECATION")
                        intent.getParcelableExtra(UsbManager.EXTRA_DEVICE)
                    }

                val intentReaderName =
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                        intent.identifier
                    } else {
                        intent.getStringExtra(EXTRA_READER_NAME)
                    }
                val reader =
                    (if (intentReaderName != null) readers[intentReaderName] else null)
                        ?: readers.values.firstOrNull { it.pendingFuture != null }

                val manager = usbManager
                val granted =
                    intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false) ||
                        (device != null && manager?.hasPermission(device) == true)

                if (granted && device != null) {
                    if (reader == null) {
                        Log.e(TAG, "Reader not found for granted USB permission")
                        return
                    }
                    val pending = reader.pendingFuture
                    if (reader.permissionOnly) {
                        readers[reader.name] =
                            reader.copy(pendingFuture = null, permissionOnly = false)
                        eventSink?.emit("usb_permission_granted", mapOf("reader" to reader.name))
                        pending?.complete(HostCallResult.Success(true))
                        return
                    }

                    val executor = worker
                    if (executor == null) {
                        readers[reader.name] = reader.copy(pendingFuture = null)
                        pending?.complete(
                            HostCallResult.Error(
                                "CCID_READER_CONNECT_ERROR",
                                "CCID worker unavailable",
                            ),
                        )
                        return
                    }

                    executor.execute {
                        try {
                            val connected = connectToInterface(device, reader.interfaceIdx)
                            if (connected != null) {
                                readers[reader.name] =
                                    reader.copy(ccid = connected.first, pendingFuture = null)
                                eventSink?.emit(
                                    "usb_permission_granted",
                                    mapOf("reader" to reader.name),
                                )
                                pending?.complete(HostCallResult.Success(connected.second))
                            } else {
                                readers[reader.name] = reader.copy(pendingFuture = null)
                                pending?.complete(
                                    HostCallResult.Error(
                                        "CCID_READER_CONNECT_ERROR",
                                        "Failed to connect [r]",
                                    ),
                                )
                            }
                        } catch (c: CcidCardNotFoundException) {
                            readers[reader.name] = reader.copy(pendingFuture = null)
                            pending?.complete(
                                HostCallResult.Error("CCID_READER_CONNECT_ERROR", "No card in reader"),
                            )
                        } catch (e: Exception) {
                            readers[reader.name] = reader.copy(pendingFuture = null)
                            pending?.complete(
                                HostCallResult.Error(
                                    "CCID_READER_CONNECT_ERROR",
                                    e.message ?: "Failed to connect",
                                ),
                            )
                        }
                    }
                } else {
                    Log.d(TAG, "permission denied for device $device")
                    if (reader != null && reader.pendingFuture != null) {
                        readers[reader.name] =
                            reader.copy(pendingFuture = null, permissionOnly = false)
                        reader.pendingFuture.complete(
                            HostCallResult.Error("CCID_PERMISSION_DENIED", "USB permission denied"),
                        )
                    }
                }
            }
        }

    fun attach(appContext: Context, sink: HostEventSink) {
        val ctx = appContext.applicationContext
        context = ctx
        usbManager = ctx.getSystemService(Context.USB_SERVICE) as UsbManager
        eventSink = sink
        if (worker == null || worker?.isShutdown == true) {
            worker = Executors.newSingleThreadExecutor()
        }

        if (!receiverRegistered) {
            val filter = IntentFilter(ACTION_USB_PERMISSION)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                ctx.registerReceiver(usbReceiver, filter, Context.RECEIVER_EXPORTED)
            } else {
                @Suppress("UnspecifiedRegisterReceiverFlag")
                ctx.registerReceiver(usbReceiver, filter)
            }
            receiverRegistered = true
        }
    }

    fun detach() {
        val ctx = context
        if (receiverRegistered && ctx != null) {
            try {
                ctx.unregisterReceiver(usbReceiver)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to unregister usbReceiver", e)
            }
            receiverRegistered = false
        }
        for (name in readers.keys.toList()) {
            val reader = readers[name] ?: continue
            reader.pendingFuture?.complete(
                HostCallResult.Error("DISCONNECTED", "CCID manager detached"),
            )
            closeReaderConnection(reader)
            readers[name] = reader.copy(ccid = null, pendingFuture = null)
        }
        readers.clear()
        readerATR.clear()
        eventSink = null
        worker?.shutdown()
        worker = null
        usbManager = null
        context = null
    }

    fun call(method: String, args: JSONObject): HostCallResult {
        return when (method) {
            "listReaders" -> HostCallResult.Success(listReaders())
            "listReaderATRs" -> HostCallResult.Success(listReadersWithATR())
            "connect" -> {
                val name = args.optString("reader")
                if (name.isEmpty()) {
                    HostCallResult.Error("INVALID_ARGUMENT", "reader is required")
                } else {
                    connect(name)
                }
            }
            "requestPermission" -> {
                val name = args.optString("reader")
                if (name.isEmpty()) {
                    HostCallResult.Error("INVALID_ARGUMENT", "reader is required")
                } else {
                    requestPermission(name)
                }
            }
            "transceive" -> {
                val name = args.optString("reader")
                val apduHex = args.optString("apduHex").ifEmpty { args.optString("capdu") }
                if (name.isEmpty() || apduHex.isEmpty()) {
                    return HostCallResult.Error("INVALID_ARGUMENT", "reader and apduHex are required")
                }
                val reader =
                    readers[name]
                        ?: return HostCallResult.Error("CCID_READER_NOT_FOUND", "Reader not found")
                val ccid =
                    reader.ccid
                        ?: return HostCallResult.Error(
                            "CCID_READER_NOT_CONNECTED",
                            "Reader not connected",
                        )
                try {
                    val resp = ccid.xfrBlock(apduHex.decodeHexBytes())
                    HostCallResult.Success(resp.toHexStringLower())
                } catch (c: CcidCardNotFoundException) {
                    closeReaderConnection(reader)
                    readers[name] = reader.copy(ccid = null)
                    HostCallResult.Error("DISCONNECTED", c.message ?: "Card not found")
                } catch (e: Exception) {
                    HostCallResult.Error(
                        "CCID_TRANSMIT_ERROR",
                        e.message ?: "CCID transceive failed",
                    )
                }
            }
            "powerCycle" -> {
                val name = args.optString("reader")
                if (name.isEmpty()) {
                    return HostCallResult.Error("INVALID_ARGUMENT", "reader is required")
                }
                val reader =
                    readers[name]
                        ?: return HostCallResult.Error("CCID_READER_NOT_FOUND", "Reader not found")
                val ccid =
                    reader.ccid
                        ?: return HostCallResult.Error(
                            "CCID_READER_NOT_CONNECTED",
                            "Reader not connected",
                        )
                try {
                    Log.d(TAG, "Powering off: ${reader.name}")
                    ccid.iccPowerOff()
                    Log.d(TAG, "Powering on: ${reader.name}")
                    var atr: String? = null
                    var lastError: Exception? = null
                    for (i in 0..10) {
                        try {
                            atr = ccid.iccPowerOn().toHexStringLower()
                            break
                        } catch (e: Exception) {
                            lastError = e
                            Log.d(TAG, "Powering on: ${reader.name} failed ($e). retrying")
                        }
                    }
                    if (atr != null) {
                        HostCallResult.Success(atr)
                    } else {
                        HostCallResult.Error(
                            "CCID_POWER_CYCLE_ERROR",
                            "Failed to power on card: ${lastError?.message}",
                        )
                    }
                } catch (e: Exception) {
                    HostCallResult.Error(
                        "CCID_POWER_CYCLE_ERROR",
                        e.message ?: "Power cycle failed",
                    )
                }
            }
            "disconnect" -> {
                val name = args.optString("reader")
                if (name.isEmpty()) {
                    return HostCallResult.Error("INVALID_ARGUMENT", "reader is required")
                }
                val reader =
                    readers[name]
                        ?: return HostCallResult.Error("CCID_READER_NOT_FOUND", "Reader not found")
                closeReaderConnection(reader)
                readers[name] = reader.copy(ccid = null)
                HostCallResult.Success(true)
            }
            else -> HostCallResult.Error("UNSUPPORTED_METHOD", "Unknown CCID method: $method")
        }
    }

    /** Releases the USB resources a reader holds, tolerating a closed handle. */
    private fun closeReaderConnection(reader: Reader) {
        val ccid = reader.ccid ?: return
        try {
            ccid.close()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to close connection for ${reader.name}: $e")
        }
    }

    private fun listReaders(): List<String> {
        val manager = usbManager ?: return emptyList()
        val readerTree = mutableMapOf<String, MutableList<Reader>>()

        manager.deviceList.values.forEach { device ->
            (0 until device.interfaceCount).forEach { i ->
                val usbInterface = device.getInterface(i)
                if (usbInterface.interfaceClass == UsbConstants.USB_CLASS_CSCID) {
                    val displayName = getDisplayName(device, usbInterface)
                    val reader = Reader(displayName, device.deviceName, i, null, null, false)
                    readerTree.getOrPut(displayName) { mutableListOf() }.add(reader)
                }
            }
        }

        readerTree.forEach { (name, list) ->
            if (!readers.containsKey(name)) {
                readers[name] = list[0]
            }
        }

        // Remove stale disconnected readers that are no longer attached to USB
        for (existing in readers.keys.toList()) {
            if (!readerTree.containsKey(existing)) {
                readers.remove(existing)?.let { closeReaderConnection(it) }
            }
        }

        return readers.keys.toList()
    }

    private fun listReadersWithATR(): Map<String, String> {
        val manager = usbManager ?: return emptyMap()
        val results = mutableMapOf<String, String>()
        val readerNames = listReaders()
        readerNames.forEach { readerName ->
            val reader = readers[readerName] ?: return@forEach
            val device = manager.deviceList[reader.deviceName]
            if (device != null) {
                if (!manager.hasPermission(device)) {
                    requestUsbPermission(device, readerName)
                    results[readerName] = "[NO_PERMISSION]"
                } else if (readerATR.containsKey(device.hashCode())) {
                    results[readerName] = readerATR[device.hashCode()] ?: ""
                } else {
                    val resp =
                        try {
                            connectToInterface(device, reader.interfaceIdx)
                        } catch (_: Exception) {
                            null
                        }
                    results[readerName] = resp?.second ?: ""
                    if (resp != null) {
                        readerATR[device.hashCode()] = resp.second
                        // Probing owns this connection; nothing keeps the handle.
                        resp.first.close()
                    }
                }
            }
        }
        return results
    }

    private fun connect(name: String): HostCallResult {
        val manager =
            usbManager ?: return HostCallResult.Error("NOT_AVAILABLE", "UsbManager unavailable")
        val reader =
            readers[name]
                ?: run {
                    listReaders()
                    readers[name]
                }
                ?: return HostCallResult.Error("CCID_READER_NOT_FOUND", "Reader not found")
        val device =
            manager.deviceList[reader.deviceName]
                ?: return HostCallResult.Error("CCID_READER_NOT_FOUND", "Reader not found")

        if (reader.ccid != null) {
            closeReaderConnection(reader)
            readers[name] = reader.copy(ccid = null)
        }

        if (!manager.hasPermission(device)) {
            val future = CompletableFuture<HostCallResult>()
            readers[name] = reader.copy(pendingFuture = future, permissionOnly = false)
            requestUsbPermission(device, name)
            return awaitPermissionFuture(name, future)
        }

        Log.d(TAG, "Permission OK. Connecting to $name")
        return try {
            val connected = connectToInterface(device, reader.interfaceIdx)
            if (connected != null) {
                readers[name] = reader.copy(ccid = connected.first)
                HostCallResult.Success(connected.second)
            } else {
                HostCallResult.Error("CCID_READER_CONNECT_ERROR", "Failed to connect [c]")
            }
        } catch (c: CcidCardNotFoundException) {
            HostCallResult.Error("CCID_READER_CONNECT_ERROR", "No card in reader")
        } catch (e: Exception) {
            HostCallResult.Error(
                "CCID_READER_CONNECT_ERROR",
                e.message ?: "Failed to connect",
            )
        }
    }

    private fun requestPermission(name: String): HostCallResult {
        val manager =
            usbManager ?: return HostCallResult.Error("NOT_AVAILABLE", "UsbManager unavailable")
        val reader =
            readers[name]
                ?: run {
                    listReaders()
                    readers[name]
                }
                ?: return HostCallResult.Error("CCID_READER_NOT_FOUND", "Reader not found")
        val device =
            manager.deviceList[reader.deviceName]
                ?: return HostCallResult.Error("CCID_READER_NOT_FOUND", "Reader not found")

        if (manager.hasPermission(device)) {
            return HostCallResult.Success(true)
        }

        val future = CompletableFuture<HostCallResult>()
        readers[name] = reader.copy(pendingFuture = future, permissionOnly = true)
        requestUsbPermission(device, name)
        return awaitPermissionFuture(name, future)
    }

    private fun requestUsbPermission(device: UsbDevice, readerName: String) {
        val appContext = context ?: return
        val manager = usbManager ?: return
        val intent =
            Intent(ACTION_USB_PERMISSION).apply {
                setPackage(appContext.packageName)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    identifier = readerName
                } else {
                    putExtra(EXTRA_READER_NAME, readerName)
                }
            }
        val flags =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            } else {
                PendingIntent.FLAG_UPDATE_CURRENT
            }
        val pendingIntent =
            PendingIntent.getBroadcast(appContext, readerName.hashCode(), intent, flags)
        manager.requestPermission(device, pendingIntent)
    }

    private fun awaitPermissionFuture(
        name: String,
        future: CompletableFuture<HostCallResult>,
    ): HostCallResult {
        return try {
            future.get(PERMISSION_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (e: Exception) {
            readers[name]?.let {
                if (it.pendingFuture === future) {
                    readers[name] = it.copy(pendingFuture = null, permissionOnly = false)
                }
            }
            HostCallResult.Error(
                "CCID_PERMISSION_TIMEOUT",
                e.message ?: "Timed out waiting for USB permission",
            )
        }
    }

    private fun connectToInterface(device: UsbDevice, interfaceIdx: Int): Pair<Ccid, String>? {
        val manager = usbManager ?: return null
        val usbInterface = device.getInterface(interfaceIdx)
        val usbConnection = manager.openDevice(device)
        if (usbConnection == null) {
            Log.e(TAG, "Failed to open device")
            return null
        }
        val endpoints =
            try {
                getEndpoints(usbInterface)
            } catch (e: Exception) {
                usbConnection.close()
                throw e
            }
        val ccid = Ccid(usbConnection, endpoints.first, endpoints.second, usbInterface)
        try {
            val descriptor = ccid.getDescriptor(interfaceIdx)
            if (descriptor?.supportsProtocol(Protocol.T0) != true) {
                Log.d(TAG, "Unsupported protocol")
                ccid.close()
                return null
            }
            if (!usbConnection.claimInterface(usbInterface, true)) {
                Log.e(TAG, "Failed to claim interface")
                ccid.close()
                return null
            }
            val atr = ccid.iccPowerOn()
            val atrHex = atr.toHexStringLower()
            Log.d(TAG, "ATR: $atrHex")
            return Pair(ccid, atrHex)
        } catch (e: Exception) {
            ccid.close()
            throw e
        }
    }

    private fun getEndpoints(usbInterface: UsbInterface): Pair<UsbEndpoint, UsbEndpoint> {
        var bulkIn: UsbEndpoint? = null
        var bulkOut: UsbEndpoint? = null
        for (i in 0 until usbInterface.endpointCount) {
            val endpoint = usbInterface.getEndpoint(i)
            if (endpoint.type == UsbConstants.USB_ENDPOINT_XFER_BULK) {
                if (endpoint.direction == UsbConstants.USB_DIR_IN) {
                    bulkIn = endpoint
                } else {
                    bulkOut = endpoint
                }
            }
        }
        if (bulkIn == null || bulkOut == null) {
            throw Exception("Bulk endpoints not found")
        }
        return Pair(bulkIn, bulkOut)
    }

    private fun getDisplayName(device: UsbDevice, usbInterface: UsbInterface): String {
        val nameParts = mutableListOf<String>()
        if (device.productName != null) {
            nameParts.add(device.productName!!)
        } else {
            nameParts.add("Unknown")
        }

        if (usbInterface.name != null) {
            nameParts.add(usbInterface.name!!)
        } else {
            nameParts.add("CCID")
        }
        return nameParts.joinToString(" ")
    }

    companion object {
        private const val TAG = "CcidPlugin"
        private const val ACTION_USB_PERMISSION = "im.nfc.ccid.USB_PERMISSION"
        private const val EXTRA_READER_NAME = "im.nfc.ccid.EXTRA_READER_NAME"
        private const val PERMISSION_TIMEOUT_SECONDS = 60L
    }

    private data class Reader(
        val name: String,
        val deviceName: String,
        val interfaceIdx: Int,
        val ccid: Ccid?,
        val pendingFuture: CompletableFuture<HostCallResult>?,
        val permissionOnly: Boolean = false,
    )
}
