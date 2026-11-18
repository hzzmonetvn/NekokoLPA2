package ee.nekoko.nlpa2

import android.content.BroadcastReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.os.Build
import android.os.DeadObjectException
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Log
import java.io.ByteArrayOutputStream
import java.util.concurrent.CompletableFuture
import java.util.concurrent.CountDownLatch
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.json.JSONObject
import rikka.shizuku.Shizuku

/**
 * The app's way to the modem's bus and Secure Element HAL.
 *
 * The socket has to be opened by the `shell` user, which is what Shizuku runs
 * [QrtrUserService] as, and it has to stay there: an app is refused a socket
 * of its own and refused the use of one this process opened. What crosses to
 * the app therefore is not a descriptor but the traffic itself — the host asks
 * for a datagram to go out, or waits for one to come in, and the answering
 * happens in the process that holds the socket.
 *
 * The SE HAL reader rides on the same service, since the Secure Element HAL
 * also answers only to `shell`. Its calls start with `seHal`, and a change of
 * SIM state is passed on to the service so that it drops channels it can no
 * longer vouch for.
 */
internal class QrtrPlugin {
    private lateinit var libraryDir: String
    private lateinit var apkPath: String
    private lateinit var abi: String

    private var worker: ExecutorService? = null
    private val main = Handler(Looper.getMainLooper())

    @Volatile private var service: IQrtrService? = null
    @Volatile private var activeBusFd: Int? = null
    private var receiverContext: Context? = null
    private val pendingInvalidations = AtomicInteger(0)
    private val simStateReceiver =
        object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                intent ?: return
                val slot =
                    if (intent.action == "android.se.omapi.action.SECURE_ELEMENT_STATE_CHANGED") {
                        when (intent.getStringExtra("android.se.omapi.extra.READER_NAME")) {
                            "SIM1" -> 1
                            "SIM2" -> 2
                            else -> return // eSE is not one of our SIM readers.
                        }
                    } else {
                        0
                    }
                val connected = service
                try {
                    if (connected == null) {
                        queueInvalidation(slot)
                    } else {
                        connected.halInvalidateSessions(slot) // No HAL init, reset or close.
                    }
                } catch (e: Throwable) {
                    queueInvalidation(slot)
                    Log.w(TAG, "SIM change invalidation deferred until next SE HAL operation", e)
                }
            }
        }

    private fun queueInvalidation(slot: Int) {
        val mask = if (slot == 0) 3 else 1 shl (slot - 1)
        pendingInvalidations.getAndUpdate { it or mask }
    }

    private fun applyPendingInvalidations(service: IQrtrService) {
        val mask = pendingInvalidations.getAndSet(0)
        try {
            if (mask == 3) {
                service.halInvalidateSessions(0)
            } else if (mask != 0) {
                service.halInvalidateSessions(if (mask == 1) 1 else 2)
            }
        } catch (e: Throwable) {
            pendingInvalidations.getAndUpdate { it or mask }
            throw e // Never perform a card operation before invalidation succeeds.
        }
    }

    @Volatile private var connecting: CountDownLatch? = null
    @Volatile private var pendingPermission: CompletableFuture<HostCallResult>? = null

    private val connection =
        object : ServiceConnection {
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                Log.i(TAG, "user service connected")
                val userService = IQrtrService.Stub.asInterface(binder)
                try {
                    userService.prepare(libraryDir, apkPath, abi)
                } catch (e: Throwable) {
                    Log.e(TAG, "the user service could not load the modem library", e)
                    connecting?.countDown()
                    return
                }
                this@QrtrPlugin.service = userService
                connecting?.countDown()
            }

            override fun onServiceDisconnected(name: ComponentName) {
                Log.i(TAG, "user service disconnected")
                service = null
            }
        }

    private val permissionListener =
        Shizuku.OnRequestPermissionResultListener { requestCode, grantResult ->
            if (requestCode != REQUEST_PERMISSION) return@OnRequestPermissionResultListener
            val granted = grantResult == PackageManager.PERMISSION_GRANTED
            Log.i(TAG, "permission answer: granted=$granted")
            pendingPermission?.let { future ->
                pendingPermission = null
                future.complete(HostCallResult.Success(granted))
            }
        }

    private val serviceArgs =
        Shizuku.UserServiceArgs(
            ComponentName(BuildConfig.APPLICATION_ID, QrtrUserService::class.java.name),
        )
            .daemon(false)
            .processNameSuffix("qrtr")
            .debuggable(BuildConfig.DEBUG)
            .version(BuildConfig.VERSION_CODE)

    fun attach(appContext: Context) {
        val ctx = appContext.applicationContext
        if (worker == null || worker?.isShutdown == true) {
            worker = Executors.newSingleThreadExecutor()
        }

        val filter =
            IntentFilter().apply {
                addAction("android.intent.action.SIM_STATE_CHANGED")
                addAction("android.telephony.action.SIM_CARD_STATE_CHANGED")
                addAction("android.telephony.action.SIM_APPLICATION_STATE_CHANGED")
                addAction("android.se.omapi.action.SECURE_ELEMENT_STATE_CHANGED")
            }
        try {
            if (Build.VERSION.SDK_INT >= 33) {
                ctx.registerReceiver(simStateReceiver, filter, Context.RECEIVER_EXPORTED)
            } else {
                @Suppress("UnspecifiedRegisterReceiverFlag")
                ctx.registerReceiver(simStateReceiver, filter)
            }
            receiverContext = ctx
        } catch (e: RuntimeException) {
            Log.e(TAG, "SE HAL state monitoring unavailable; direct access disabled", e)
        }

        val application = ctx.applicationInfo
        libraryDir = application.nativeLibraryDir
        apkPath = application.sourceDir
        abi = Build.SUPPORTED_ABIS.firstOrNull() ?: "arm64-v8a"

        try {
            Shizuku.addRequestPermissionResultListener(permissionListener)
        } catch (e: Throwable) {
            Log.w(TAG, "Shizuku is not available", e)
        }
    }

    fun detach() {
        try {
            Shizuku.removeRequestPermissionResultListener(permissionListener)
            Shizuku.unbindUserService(serviceArgs, connection, true)
        } catch (e: Throwable) {
            Log.w(TAG, "could not let the user service go", e)
        }

        pendingPermission?.complete(HostCallResult.Success(false))
        pendingPermission = null

        try {
            receiverContext?.unregisterReceiver(simStateReceiver)
        } catch (_: Throwable) {}
        receiverContext = null
        service = null
        activeBusFd = null
        worker?.shutdown()
        worker = null
    }

    fun call(op: String, args: JSONObject): HostCallResult {
        return when (op) {
            "qrtr.isSupported" -> HostCallResult.Success(isQualcomm())
            "qrtr.isAvailable" -> HostCallResult.Success(isAvailable())
            "qrtr.hasPermission" -> HostCallResult.Success(hasPermission())
            "qrtr.requestPermission" -> requestPermission()
            "qrtr.openBus" ->
                withService { svc ->
                    val fd = svc.openBus()
                    activeBusFd = fd
                    fd
                }
            "qrtr.busNode" ->
                withService { svc ->
                    val fd = resolveBusFd(args)
                    svc.busNode(fd)
                }
            "qrtr.send" ->
                withService { svc ->
                    val fd = resolveBusFd(args)
                    val node = args.requireInt("node")
                    val port = args.requireInt("port")
                    val payloadHex = args.requireNonEmptyString("payloadHex")
                    svc.send(fd, node, port, payloadHex.decodeHexBytes())
                    true
                }
            "qrtr.receive" ->
                withService { svc ->
                    val fd = resolveBusFd(args)
                    val timeoutMs = args.optInt("timeoutMs", 5000)
                    val framed = svc.receive(fd, timeoutMs)
                    if (framed == null || framed.size < 8) {
                        null
                    } else {
                        val node = readLittleEndianU32(framed, 0)
                        val port = readLittleEndianU32(framed, 4)
                        val payloadHex = framed.copyOfRange(8, framed.size).toHexStringLower()
                        mapOf("node" to node, "port" to port, "payloadHex" to payloadHex)
                    }
                }
            "qrtr.closeBus" ->
                withService { svc ->
                    val fd = resolveBusFd(args)
                    svc.closeBus(fd)
                    if (activeBusFd == fd) {
                        activeBusFd = null
                    }
                    true
                }
            "seHal.invalidateSessions" ->
                withService(retrySafe = false, seHal = true) { svc ->
                    svc.halInvalidateSessions(args.optInt("slot", 0))
                    true
                }
            "seHal.useBorrowed" ->
                withService(retrySafe = false, seHal = true) { svc ->
                    svc.halUseBorrowed(args.requireInt("slot"))
                    true
                }
            "seHal.openOwned" ->
                withService(retrySafe = false, seHal = true) { svc ->
                    svc.halOpenOwned(args.requireInt("slot"))
                    true
                }
            "seHal.closeOwned" ->
                withService(retrySafe = false, seHal = true) { svc ->
                    svc.halCloseOwned(args.requireInt("slot"))
                    true
                }
            "seHal.atr" ->
                withService(seHal = true) { svc ->
                    svc.halAtr(args.requireInt("slot")).toHexStringLower()
                }
            "seHal.read" ->
                withService(seHal = true) { svc ->
                    val slot = args.requireInt("slot")
                    val apduHex =
                        args.optString("apduHex").ifEmpty { args.requireNonEmptyString("dataHex") }
                    svc.halRead(slot, apduHex.decodeHexBytes()).toHexStringLower()
                }
            "seHal.beginDownload" ->
                withService(seHal = true) { svc ->
                    svc.halBeginDownload(args.requireInt("slot"))
                    true
                }
            "seHal.endDownload" ->
                withService(seHal = true) { svc ->
                    svc.halEndDownload(args.requireInt("slot"))
                    true
                }
            "seHal.download" ->
                withService(retrySafe = false, seHal = true, timeoutSeconds = 120L) { svc ->
                    val slot = args.requireInt("slot")
                    val (payload, segments) = parseDownloadPayload(args)
                    require(payload.size in 3..(16 * 1024 * 1024)) { "Invalid ES10b size" }
                    require(segments.size in 1..65536) { "Invalid segment count" }
                    // Binder has a shared ~1 MiB transaction buffer. Transfer the
                    // package as small data-only chunks, then validate and send it
                    // inside one native call. Staging never talks to the card.
                    try {
                        var offset = 0
                        while (offset < payload.size) {
                            val end = minOf(offset + 128 * 1024, payload.size)
                            svc.halStageDownload(
                                slot,
                                payload.copyOfRange(offset, end),
                                offset == 0,
                            )
                            offset = end
                        }
                        svc.halDownload(slot, byteArrayOf(), segments).toHexStringLower()
                    } catch (e: Throwable) {
                        try {
                            svc.halEndDownload(slot)
                        } catch (_: Throwable) {}
                        throw e
                    }
                }
            "seHal.switchProfile" ->
                withService(retrySafe = false, seHal = true, switchOperation = true) { svc ->
                    val slot = args.requireInt("slot")
                    val iccid = args.requireNonEmptyString("iccidHex").decodeHexBytes()
                    val eid = optionalHex(args, "eidHex") ?: readHalEidBytes(svc, slot)
                    val enable =
                        if (args.has("enable")) {
                            args.getBoolean("enable")
                        } else {
                            invalid("enable")
                        }
                    svc.halSwitchProfile(slot, iccid, eid, enable).toHexStringLower()
                }
            "seHal.renameProfile" ->
                withService(retrySafe = false, seHal = true) { svc ->
                    val slot = args.requireInt("slot")
                    val iccid = args.requireNonEmptyString("iccidHex").decodeHexBytes()
                    val eid = optionalHex(args, "eidHex") ?: readHalEidBytes(svc, slot)
                    val nickname = args.getString("nickname").toByteArray(Charsets.UTF_8)
                    svc.halRenameProfile(slot, iccid, eid, nickname).toHexStringLower()
                }
            "seHal.deleteProfile" ->
                withService(retrySafe = false, seHal = true) { svc ->
                    val slot = args.requireInt("slot")
                    val iccid = args.requireNonEmptyString("iccidHex").decodeHexBytes()
                    val eid = optionalHex(args, "eidHex") ?: readHalEidBytes(svc, slot)
                    svc.halDeleteProfile(slot, iccid, eid).toHexStringLower()
                }
            "seHal.removeNotification" ->
                withService(retrySafe = false, seHal = true) { svc ->
                    val slot = args.requireInt("slot")
                    val seq = args.requireInt("seqNumber")
                    val eid = optionalHex(args, "eidHex") ?: readHalEidBytes(svc, slot)
                    val expectedResponse =
                        optionalHex(args, "expectedResponseHex")
                            ?: optionalHex(args, "responseHex")
                            ?: readHalNotificationSnapshot(svc, slot, seq)
                    svc.halRemoveNotification(slot, seq, eid, expectedResponse).toHexStringLower()
                }
            else -> HostCallResult.Error("UNSUPPORTED_METHOD", "Unknown QRTR/SE-HAL op: $op")
        }
    }

    private fun parseDownloadPayload(args: JSONObject): Pair<ByteArray, IntArray> {
        val segmentsHex = args.optJSONArray("segmentsHex")
        if (segmentsHex != null && segmentsHex.length() > 0) {
            val stream = ByteArrayOutputStream()
            val lengths = IntArray(segmentsHex.length())
            for (i in 0 until segmentsHex.length()) {
                val segBytes = segmentsHex.getString(i).decodeHexBytes()
                lengths[i] = segBytes.size
                stream.write(segBytes)
            }
            return stream.toByteArray() to lengths
        }
        val payloadHex = args.optString("payloadHex")
        if (payloadHex.isNotEmpty()) {
            val payload = payloadHex.decodeHexBytes()
            val segArr = args.optJSONArray("segments")
            val lengths =
                if (segArr != null && segArr.length() > 0) {
                    IntArray(segArr.length()) { i -> segArr.getInt(i) }
                } else {
                    intArrayOf(payload.size)
                }
            return payload to lengths
        }
        invalid("segmentsHex")
    }

    private fun optionalHex(args: JSONObject, key: String): ByteArray? =
        args.optString(key).takeIf { it.isNotEmpty() }?.decodeHexBytes()

    private fun readHalEidBytes(service: IQrtrService, slot: Int): ByteArray {
        val apdu = byteArrayOf(0x81.toByte(), 0xE2.toByte(), 0x91.toByte(), 0, 6, 0xBF.toByte(), 0x3E, 3, 0x5C, 1, 0x5A)
        val resp = halReadFull(service, slot, apdu)
        check(
            resp.size == 23 &&
                resp[0] == 0xBF.toByte() &&
                resp[1] == 0x3E.toByte() &&
                resp[2] == 0x12.toByte() &&
                resp[3] == 0x5A.toByte() &&
                resp[4] == 0x10.toByte() &&
                resp[21] == 0x90.toByte() &&
                resp[22] == 0.toByte(),
        ) { "Could not read current EID on SE HAL slot $slot" }
        return resp.copyOfRange(5, 21)
    }

    private fun readHalNotificationSnapshot(
        service: IQrtrService,
        slot: Int,
        seqNumber: Int,
    ): ByteArray {
        val req = halNotificationRequest(0xBF2B, seqNumber)
        require(req.size <= 255) { "Notification request too large" }
        val apdu =
            byteArrayOf(
                0x81.toByte(),
                0xE2.toByte(),
                0x91.toByte(),
                0x00,
                req.size.toByte(),
            ) + req
        val resp = halReadFull(service, slot, apdu)
        check(
            resp.size >= 2 &&
                resp[resp.lastIndex - 1] == 0x90.toByte() &&
                resp.last() == 0.toByte(),
        ) { "Notification snapshot read failed" }
        return resp.copyOfRange(0, resp.size - 2)
    }

    private fun halReadFull(service: IQrtrService, slot: Int, apdu: ByteArray): ByteArray {
        var cmd = apdu
        val out = ByteArrayOutputStream()
        repeat(2048) {
            val r = service.halRead(slot, cmd)
            check(r.size >= 2) { "Short SE HAL reply" }
            out.write(r, 0, r.size - 2)
            val sw1 = r[r.lastIndex - 1].toInt() and 255
            val sw2 = r.last().toInt() and 255
            if (sw1 != 0x61) {
                out.write(sw1)
                out.write(sw2)
                return out.toByteArray()
            }
            cmd = byteArrayOf(0x81.toByte(), 0xC0.toByte(), 0, 0, sw2.toByte())
        }
        error("Too many GET RESPONSEs during halRead")
    }

    private fun resolveBusFd(args: JSONObject): Int {
        if (args.has("fd") && !args.isNull("fd")) {
            return args.getInt("fd")
        }
        return activeBusFd ?: throw IllegalStateException("No open QRTR bus")
    }

    private fun readLittleEndianU32(bytes: ByteArray, offset: Int): Long =
        ((bytes[offset].toLong() and 0xFF)) or
            ((bytes[offset + 1].toLong() and 0xFF) shl 8) or
            ((bytes[offset + 2].toLong() and 0xFF) shl 16) or
            ((bytes[offset + 3].toLong() and 0xFF) shl 24)

    private fun JSONObject.requireInt(name: String): Int {
        if (!has(name) || isNull(name)) invalid(name)
        return getInt(name)
    }

    private fun JSONObject.requireNonEmptyString(name: String): String {
        val v = optString(name, "")
        if (v.isEmpty()) invalid(name)
        return v
    }

    private fun invalid(name: String): Nothing = throw IllegalArgumentException("$name is missing")

    /** Run `call` against the user service, off the calling thread. */
    private fun withService(
        retrySafe: Boolean = true,
        seHal: Boolean = false,
        switchOperation: Boolean = false,
        timeoutSeconds: Long = CALL_TIMEOUT_SECONDS,
        call: (IQrtrService) -> Any?,
    ): HostCallResult {
        if (seHal && receiverContext == null) {
            return HostCallResult.Error(
                "NO_STATE_MONITOR",
                "SE HAL state monitoring is unavailable",
            )
        }
        if (!hasPermission()) {
            return HostCallResult.Error("NO_PERMISSION", "Shizuku permission was not granted")
        }
        val executor =
            worker ?: return HostCallResult.Error("NO_SERVICE", "QRTR worker unavailable")

        val future = CompletableFuture<HostCallResult>()
        executor.execute {
            var attempt = 0
            while (true) {
                val svc =
                    try {
                        connect()
                    } catch (e: Throwable) {
                        future.complete(failResult("NO_SERVICE", e))
                        return@execute
                    }

                try {
                    if (seHal) applyPendingInvalidations(svc)
                    val value = call(svc)
                    future.complete(HostCallResult.Success(value))
                    return@execute
                } catch (e: Throwable) {
                    Log.w(TAG, "the user service could not answer", e)

                    if (!retrySafe || e !is DeadObjectException) {
                        if (switchOperation && e is DeadObjectException) {
                            forget(svc)
                            future.complete(failResult("HAL_SWITCH_UNCERTAIN", e))
                        } else {
                            future.complete(failResult("SERVICE_ERROR", e))
                        }
                        return@execute
                    }

                    forget(svc)
                    if (++attempt >= MAX_ATTEMPTS) {
                        future.complete(failResult("NO_SERVICE", e))
                        return@execute
                    }
                    Thread.sleep(RETRY_DELAY_MS)
                }
            }
        }
        return try {
            future.get(timeoutSeconds, TimeUnit.SECONDS)
        } catch (e: Exception) {
            HostCallResult.Error("SERVICE_ERROR", e.message ?: "QRTR/SE-HAL operation timed out")
        }
    }

    /** The user service, waiting for it to come up if it is not there yet. */
    private fun connect(): IQrtrService {
        service?.let {
            return it
        }

        val latch = CountDownLatch(1)
        connecting = latch
        main.post { bind() }

        if (!latch.await(BIND_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
            connecting = null
            throw IllegalStateException("the QRTR user service did not start")
        }

        connecting = null
        return service ?: throw IllegalStateException("the QRTR user service is not available")
    }

    private fun bind() {
        try {
            Shizuku.bindUserService(serviceArgs, connection)
        } catch (e: Throwable) {
            Log.w(TAG, "could not start the QRTR user service", e)
            connecting?.countDown()
        }
    }

    /** Let a service that did not answer go, so the next call binds a new one. */
    private fun forget(service: IQrtrService) {
        if (this.service !== service) return
        this.service = null
        try {
            Shizuku.unbindUserService(serviceArgs, connection, true)
        } catch (e: Throwable) {
            Log.w(TAG, "could not let the stale user service go", e)
        }
    }

    private fun failResult(code: String, error: Throwable): HostCallResult.Error {
        val message = error.message?.takeIf { it.isNotBlank() } ?: error.toString()
        return HostCallResult.Error(code, message)
    }

    /** Whether Shizuku is installed, running, and new enough for user services. */
    private fun isAvailable(): Boolean {
        return try {
            Shizuku.pingBinder() && Shizuku.getVersion() >= MIN_VERSION
        } catch (e: Throwable) {
            Log.w(TAG, "Shizuku is not available", e)
            false
        }
    }

    private fun isQualcomm(): Boolean {
        val hardware = Build.HARDWARE
        val socManufacturer =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) Build.SOC_MANUFACTURER else ""

        return hardware.contains("qcom", ignoreCase = true) ||
            socManufacturer.contains("qcom", ignoreCase = true) ||
            socManufacturer.contains("qualcomm", ignoreCase = true)
    }

    private fun hasPermission(): Boolean {
        return try {
            Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED
        } catch (e: Throwable) {
            Log.w(TAG, "Shizuku is not available", e)
            false
        }
    }

    private fun requestPermission(): HostCallResult {
        if (hasPermission()) {
            return HostCallResult.Success(true)
        }
        if (!isAvailable()) {
            return HostCallResult.Error("NO_SHIZUKU", "Shizuku is not running")
        }

        val future = CompletableFuture<HostCallResult>()
        pendingPermission?.complete(HostCallResult.Success(false))
        pendingPermission = future

        try {
            Shizuku.requestPermission(REQUEST_PERMISSION)
        } catch (e: Throwable) {
            pendingPermission = null
            Log.w(TAG, "could not ask Shizuku for permission", e)
            return HostCallResult.Error(
                "NO_PERMISSION_REQUEST",
                e.message ?: "Failed to request Shizuku permission",
            )
        }

        return try {
            future.get(PERMISSION_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (e: Exception) {
            if (pendingPermission === future) pendingPermission = null
            HostCallResult.Error(
                "PERMISSION_TIMEOUT",
                e.message ?: "Timed out waiting for Shizuku permission",
            )
        }
    }

    companion object {
        private const val TAG = "QrtrPlugin"
        private const val REQUEST_PERMISSION = 0x7271
        private const val MIN_VERSION = 10
        private const val BIND_TIMEOUT_MS = 20_000L
        private const val MAX_ATTEMPTS = 3
        private const val RETRY_DELAY_MS = 250L
        private const val CALL_TIMEOUT_SECONDS = 60L
        private const val PERMISSION_TIMEOUT_SECONDS = 60L
    }
}
