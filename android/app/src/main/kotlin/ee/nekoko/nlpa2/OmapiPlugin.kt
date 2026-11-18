package ee.nekoko.nlpa2

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.SystemClock
import android.se.omapi.Channel
import android.se.omapi.Reader
import android.se.omapi.SEService
import android.se.omapi.Session
import android.util.Log
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.TimeUnit
import org.json.JSONObject

internal class OmapiPlugin {
    private var context: Context? = null
    private var eventSink: HostEventSink? = null
    private var seService: SEService? = null
    private var serviceReadiness: OmapiServiceReadiness? = null
    private var receiverRegistered = false

    // Track open sessions and channels per reader - using thread-safe maps
    private val readerSessions = ConcurrentHashMap<String, Session>()
    private val readerChannels = ConcurrentHashMap<String, ConcurrentHashMap<String, Channel>>()

    // Cache for reader status - using thread-safe map
    private val readerStatusCache = ConcurrentHashMap<String, String>()
    @Volatile private var lastScanTime = 0L

    // Track readers that have successfully opened at least one logical channel
    private val readersWithSuccessfulChannel = ConcurrentHashMap.newKeySet<String>()

    @Volatile private var profileSwitchInProgress = false

    private lateinit var cleanupCoordinator: OmapiCleanupCoordinator<Channel>
    private var lifecycleCoordinator: OmapiLifecycleCoordinator? = null

    private fun createCleanupCoordinator(appContext: Context): OmapiCleanupCoordinator<Channel> =
        OmapiCleanupCoordinator(
            backend =
                object : OmapiCleanupBackend<Channel> {
                    override fun closeChannel(channel: Channel) = channel.close()

                    override fun closeSessionChannels(readerName: String): Nothing =
                        error("Session.closeChannels() is forbidden by the Samsung workaround")

                    override fun closeSession(readerName: String): Nothing =
                        error("Session.close() is forbidden by the Samsung workaround")

                    override fun closeReaderSessions(readerName: String): Nothing =
                        error("Reader.closeSessions() is forbidden by the Samsung workaround")

                    override fun reconnectService(): Nothing =
                        error("SEService reconnect is forbidden after poisoning")
                },
            readerKeys = {
                (readerSessions.keys + readerChannels.keys + readersWithSuccessfulChannel).toSet()
            },
            detachChannel = { readerName, channelKey ->
                readerChannels[readerName]?.remove(channelKey.uppercase())
            },
            detachReader = { readerName ->
                // Detach every local reference before making a remote Binder call.
                readerSessions.remove(readerName)
                readersWithSuccessfulChannel.remove(readerName)
                readerChannels.remove(readerName)?.values?.toList() ?: emptyList()
            },
            clearAllLocalState = {
                readerSessions.clear()
                readerChannels.clear()
                readersWithSuccessfulChannel.clear()
            },
            safetyStore = SharedPreferencesOmapiSafetyStore(appContext),
            bootIdentityProvider = AndroidOmapiBootIdentityProvider(appContext),
            bootedAtEpochMillis = { System.currentTimeMillis() - SystemClock.elapsedRealtime() },
            hasOpenChannels = { readerChannels.values.any { it.isNotEmpty() } },
        )

    // Dedicated background thread for hardware operations
    private var backgroundHandler: Handler? = null
    private var backgroundThread: HandlerThread? = null

    // SIM state receiver to handle proactive refreshes transparently
    private val simStateReceiver =
        object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                val action = intent?.action ?: return
                if (action == "android.intent.action.SIM_STATE_CHANGED") {
                    val state = intent.getStringExtra("ss") ?: ""
                    Log.d(TAG, "SIM state changed: $state")
                    lifecycleCoordinator?.enqueueHardware(
                        onRejected = {},
                        operation = {
                            Log.i(
                                TAG,
                                "SIM state change detected ($state), cleaning up stale sessions/channels",
                            )
                            if (cleanupCoordinator.poisonInfo == null) {
                                val cleanup = cleanupAllSessions()
                                if (cleanup is OmapiCleanupResult.RebootRequired) {
                                    Log.e(
                                        TAG,
                                        "SIM refresh cleanup poisoned OMAPI; reboot is required",
                                    )
                                }
                            }

                            eventSink?.emit("sim_state_changed", mapOf("state" to state))
                        },
                    )
                }
            }
        }

    companion object {
        private const val TAG = "OmapiPlugin"
        private const val CALL_TIMEOUT_SECONDS = 90L
        private const val CORRUPTED_MESSAGE =
            "SIM/eSIM channel became invalid. The profile operation may already have taken effect. " +
                "Restart the device, reopen the app, and refresh profile status before retrying."

        // Default eUICC AID
        private const val DEFAULT_EUICC_AID = "A0000005591010FFFFFFFF8900000100"
    }

    fun attach(appContext: Context, sink: HostEventSink) {
        Log.i(TAG, "attach")
        // Restore reboot-required state before registering any callable hardware entry point.
        val ctx = appContext.applicationContext
        context = ctx
        eventSink = sink
        cleanupCoordinator = createCleanupCoordinator(ctx)
        cleanupCoordinator.poisonInfo?.let {
            Log.e(
                TAG,
                "Persisted OMAPI poison restored; hardware access remains blocked until reboot",
            )
        }

        if (!receiverRegistered) {
            val filter = IntentFilter("android.intent.action.SIM_STATE_CHANGED")
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                ctx.registerReceiver(simStateReceiver, filter, Context.RECEIVER_EXPORTED)
            } else {
                @Suppress("UnspecifiedRegisterReceiverFlag")
                ctx.registerReceiver(simStateReceiver, filter)
            }
            receiverRegistered = true
        }

        startBackgroundThread()
        val queued =
            lifecycleCoordinator?.attach {
                try {
                    initializeSEService().also { if (it) checkpointCleanIdle() }
                } catch (e: Exception) {
                    Log.e(TAG, "Failed to initialize SEService during attach", e)
                    false
                }
            } == true
        if (!queued) Log.e(TAG, "Unable to queue OMAPI attach")
    }

    fun detach() {
        Log.i(TAG, "detach")
        val ctx = context
        if (receiverRegistered && ctx != null) {
            try {
                ctx.unregisterReceiver(simStateReceiver)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to unregister simStateReceiver", e)
            }
            receiverRegistered = false
        }
        eventSink = null

        val queued =
            lifecycleCoordinator?.detach {
                try {
                    if (::cleanupCoordinator.isInitialized) {
                        shutdownServiceAfterCleanup("plugin detach")
                    }
                } finally {
                    stopBackgroundThread()
                    context = null
                }
            } == true
        if (!queued) {
            Log.e(TAG, "Unable to queue OMAPI detach cleanup; durable guard remains armed")
            stopBackgroundThread()
            context = null
        }
    }

    private fun startBackgroundThread() {
        if (backgroundThread == null) {
            val thread = HandlerThread("OmapiPluginBackend").apply { start() }
            backgroundThread = thread
            backgroundHandler = Handler(thread.looper)
            lifecycleCoordinator =
                OmapiLifecycleCoordinator { task ->
                    backgroundHandler?.post { task() } == true
                }
        }
    }

    private fun stopBackgroundThread() {
        lifecycleCoordinator = null
        backgroundThread?.quitSafely()
        backgroundThread = null
        backgroundHandler = null
    }

    private fun initializeSEService(wait: Boolean = false): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) {
            Log.w(TAG, "OMAPI requires Android 9.0 or higher")
            return false
        }

        val appContext = context ?: return false
        if (cleanupCoordinator.enterHardware(OmapiHardwareEntry.INITIALIZE_SERVICE) != null) {
            Log.e(TAG, "Refusing to initialize SEService without a durable OMAPI safety guard")
            return false
        }

        val readiness = OmapiServiceReadiness()
        serviceReadiness = readiness
        seService =
            SEService(appContext, { obj: Runnable -> obj.run() }) {
                Log.d(TAG, "SE service connected")
                eventSink?.emit("se_service_connected")
                readiness.onConnected()
            }

        if (wait && !readiness.awaitConnected({ seService?.isConnected == true })) {
            Log.w(TAG, "Timeout waiting for SE service connection")
        }
        return true
    }

    /** Called only on the serialized backend, after hardware work has returned. */
    private fun checkpointCleanIdle() {
        if (
            cleanupCoordinator.poisonInfo != null ||
                profileSwitchInProgress ||
                readerChannels.values.any { it.isNotEmpty() }
        ) {
            return
        }
        // Keep verified-empty sessions reusable; do not invoke bulk-close methods.
        cleanupCoordinator.confirmCleanIdle()?.let {
            Log.e(TAG, "Unable to confirm clean OMAPI idle state: ${it.reason}")
        }
    }

    private fun shutdownServiceAfterCleanup(reason: String): Boolean {
        val cleanup = cleanupAllSessions()
        if (cleanup is OmapiCleanupResult.RebootRequired) {
            Log.e(TAG, "Skipping SEService.shutdown() during $reason because OMAPI is poisoned")
            return false
        }

        try {
            seService?.shutdown()
            seService = null
        } catch (e: Exception) {
            cleanupCoordinator.markPoisoned(
                readerName = null,
                reason = "SEService.shutdown() failed during $reason: ${e.omapiReason()}",
                operationMayHaveSucceeded = true,
            )
            Log.e(TAG, "SEService.shutdown() failed during $reason", e)
            return false
        }

        val unsafe = cleanupCoordinator.confirmCleanShutdown()
        if (unsafe != null) {
            Log.e(TAG, "OMAPI safety guard could not be cleared during $reason")
            return false
        }
        return true
    }

    fun call(method: String, args: JSONObject): HostCallResult {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) {
            return HostCallResult.Error("UNSUPPORTED", "OMAPI requires Android 9.0 or higher")
        }

        if (method == "setProfileSwitchInProgress") {
            profileSwitchInProgress = args.optBoolean("value", false)
            if (!profileSwitchInProgress) {
                lifecycleCoordinator?.enqueueHardware(
                    onRejected = {},
                    operation = { checkpointCleanIdle() },
                )
            }
            return HostCallResult.Success(true)
        }

        val entry =
            when (method) {
                "listReaders" -> OmapiHardwareEntry.LIST_READERS
                "connect" -> OmapiHardwareEntry.CONNECT
                "openChannel" -> OmapiHardwareEntry.OPEN_CHANNEL
                "transmit", "transmitOnChannel" -> OmapiHardwareEntry.TRANSMIT
                "reset" -> OmapiHardwareEntry.RESET
                "disconnect", "closeChannel", "closeChannels" -> OmapiHardwareEntry.DISCONNECT
                else -> OmapiHardwareEntry.OPEN_SESSION
            }
        // This is deliberately a pure poison probe. Durable ARMED is established only after
        // lifecycle admission, inside the background operation immediately before hardware access.
        alreadyPoisonedResult(entry)?.let {
            return it
        }

        val backgroundMethods =
            hashSetOf(
                "connect",
                "transmit",
                "openChannel",
                "closeChannel",
                "closeChannels",
                "transmitOnChannel",
                "disconnect",
                "reset",
                "listReaders",
            )
        if (!backgroundMethods.contains(method)) {
            return HostCallResult.Error("UNSUPPORTED_METHOD", "Unknown OMAPI method: $method")
        }

        val future = CompletableFuture<HostCallResult>()
        val queued =
            lifecycleCoordinator?.enqueueHardware(
                onRejected = {
                    future.complete(
                        HostCallResult.Error("NOT_CONNECTED", "OMAPI lifecycle is detached"),
                    )
                },
                operation = {
                    try {
                        val result =
                            when (method) {
                                "connect" -> handleConnect(args)
                                "disconnect" -> handleDisconnect(args)
                                "reset" -> handleReset()
                                "transmit" -> handleTransmit(args)
                                "openChannel" -> handleOpenChannel(args, false)
                                "closeChannel" -> handleCloseChannel(args)
                                "closeChannels" -> handleCloseChannels(args)
                                "transmitOnChannel" -> handleTransmitOnChannel(args)
                                "listReaders" -> handleListReaders()
                                else ->
                                    HostCallResult.Error(
                                        "UNSUPPORTED_METHOD",
                                        "Unknown OMAPI method: $method",
                                    )
                            }
                        checkpointCleanIdle()
                        future.complete(result)
                    } catch (e: Exception) {
                        Log.e(TAG, "Error in background method: $method", e)
                        future.complete(HostCallResult.Error("ERROR", e.message ?: "OMAPI error"))
                    }
                },
            ) == true
        if (!queued) {
            Log.e(TAG, "Background handler not available for method: $method")
            return HostCallResult.Error("ERROR", "Background thread not available")
        }

        return try {
            future.get(CALL_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (e: Exception) {
            HostCallResult.Error("ERROR", e.message ?: "OMAPI call timed out")
        }
    }

    private fun alreadyPoisonedResult(entry: OmapiHardwareEntry): HostCallResult.Error? {
        val info = cleanupCoordinator.rejectionForHardwareEntry(entry) ?: return null
        return HostCallResult.Error(
            OMAPI_SESSION_CORRUPTED,
            CORRUPTED_MESSAGE,
            poisonDetails(info),
        )
    }

    private fun enterPoisonedResult(
        entry: OmapiHardwareEntry = OmapiHardwareEntry.OPEN_SESSION,
    ): HostCallResult.Error? {
        val info = cleanupCoordinator.enterHardware(entry) ?: return null
        return HostCallResult.Error(
            OMAPI_SESSION_CORRUPTED,
            CORRUPTED_MESSAGE,
            poisonDetails(info),
        )
    }

    private fun poisonDetails(info: OmapiPoisonInfo): Map<String, Any?> =
        mapOf(
            "rebootRequired" to true,
            "operationMayHaveSucceeded" to info.operationMayHaveSucceeded,
            "persistenceConfirmed" to info.persistenceConfirmed,
            "reader" to info.readerName,
            "reason" to info.reason,
        )

    private fun cleanupFailureResult(cleanup: OmapiCleanupResult): HostCallResult.Error? {
        if (cleanup !is OmapiCleanupResult.RebootRequired) return null
        return HostCallResult.Error(
            OMAPI_SESSION_CORRUPTED,
            CORRUPTED_MESSAGE,
            poisonDetails(cleanup.info),
        )
    }

    private fun trackNewChannelOrReject(
        readerName: String,
        aidHex: String,
        channel: Channel,
    ): HostCallResult.Error? {
        val key = aidHex.uppercase()
        val channels = readerChannels.getOrPut(readerName) { ConcurrentHashMap() }
        val existing = channels.putIfAbsent(key, channel)
        if (existing == null) return null

        Log.e(TAG, "Refusing to overwrite tracked channel for AID $aidHex on $readerName")
        try {
            channel.close()
        } catch (closeError: Exception) {
            val info =
                cleanupCoordinator.markPoisoned(
                    readerName,
                    closeError.omapiReason(),
                    operationMayHaveSucceeded = profileSwitchInProgress,
                )
            return HostCallResult.Error(
                OMAPI_SESSION_CORRUPTED,
                CORRUPTED_MESSAGE,
                poisonDetails(info),
            )
        }

        return HostCallResult.Error(
            "CHANNEL_ALREADY_OPEN",
            "A logical channel is already tracked for AID $aidHex",
        )
    }

    private fun reportTransmitException(
        readerName: String,
        channelKey: String,
        error: Exception,
    ): HostCallResult {
        if (profileSwitchInProgress) {
            val info =
                cleanupCoordinator.markPoisoned(
                    readerName,
                    error.omapiReason(),
                    operationMayHaveSucceeded = true,
                )
            return HostCallResult.Error(
                OMAPI_SESSION_CORRUPTED,
                CORRUPTED_MESSAGE,
                poisonDetails(info),
            )
        }

        val cleanup =
            cleanupCoordinator.cleanupChannel(
                readerName,
                channelKey.uppercase(),
                operationMayHaveSucceeded = false,
            )
        cleanupFailureResult(cleanup)?.let {
            return it
        }
        return HostCallResult.Error("TRANSMIT_FAILED", error.message ?: "Transmit failed")
    }

    private fun handleListReaders(): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val readiness = serviceReadiness
        if (seService == null || readiness?.awaitConnected({ seService?.isConnected == true }) != true) {
            return HostCallResult.Error("NOT_CONNECTED", "SE Service not connected")
        }

        val currentTime = System.currentTimeMillis()
        val allReaders = seService!!.readers.filter { it.name.startsWith("SIM") }

        val currentNames = allReaders.map { it.name }.toSet()
        val cachedNames = readerStatusCache.keys

        if (currentNames != cachedNames || currentTime - lastScanTime > 10000) {
            return try {
                // Reader discovery must not create a disposable Session: Session.close()
                // calls closeChannels() internally on affected Samsung firmware.
                val results =
                    allReaders.map { reader ->
                        if (cleanupCoordinator.poisonInfo != null) {
                            throw IllegalStateException(OMAPI_SESSION_CORRUPTED)
                        }
                        if (!reader.isSecureElementPresent) {
                            reader.name to "No SIM card|Card not detected in slot"
                        } else {
                            reader.name to null
                        }
                    }
                readerStatusCache.clear()
                for (res in results) {
                    if (res.second != null) {
                        readerStatusCache[res.first] = res.second!!
                    }
                }
                lastScanTime = currentTime
                HostCallResult.Success(formatReaderList(allReaders))
            } catch (e: Exception) {
                enterPoisonedResult()?.let {
                    return it
                }
                Log.e(TAG, "Background probing failed", e)
                HostCallResult.Success(allReaders.map { it.name })
            }
        }
        return HostCallResult.Success(formatReaderList(allReaders))
    }

    private fun formatReaderList(allReaders: List<Reader>): List<String> =
        allReaders.map { reader ->
            val errorInfo = readerStatusCache[reader.name]
            if (errorInfo != null) "${reader.name}|$errorInfo" else reader.name
        }

    private fun normalizeReaderName(readerName: String): String {
        val trimmed = readerName.trim()
        val stripped = if (trimmed.startsWith("omapi:")) trimmed.substring(6) else trimmed
        return stripped.substringBefore('|').substringBefore(" (").trim()
    }

    private fun handleConnect(args: JSONObject): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val rawReaderName =
            args.optString("reader").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "reader name required")
        val readerName = normalizeReaderName(rawReaderName)

        if (seService == null || !seService!!.isConnected) {
            return HostCallResult.Error("NOT_CONNECTED", "SE Service not connected")
        }

        if (!readerName.startsWith("SIM")) {
            return HostCallResult.Error("INVALID_READER", "Only SIM readers are supported")
        }

        val reader =
            seService!!.readers.firstOrNull { it.name == readerName }
                ?: return HostCallResult.Error("READER_NOT_FOUND", "Reader $readerName not found")

        return try {
            val cleanup = cleanupReaderSessions(readerName)
            cleanupFailureResult(cleanup)?.let {
                return it
            }

            val session = reader.openSession()
            readerSessions[readerName] = session
            readerChannels[readerName] = ConcurrentHashMap()

            val atr = session.atr?.toHexStringLower() ?: ""
            Log.i(TAG, "Connected to reader: $readerName, ATR: $atr")

            HostCallResult.Success(mapOf("success" to true, "atr" to atr))
        } catch (e: SecurityException) {
            Log.e(TAG, "Security exception connecting to reader: $readerName", e)
            HostCallResult.Error("SECURITY_EXCEPTION", "Access denied by ARA-M: ${e.message}")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to connect to reader: $readerName", e)
            HostCallResult.Error("CONNECTION_FAILED", e.message ?: "Connection failed")
        }
    }

    private fun getOrCreateSession(readerName: String): Session? {
        if (cleanupCoordinator.poisonInfo != null) return null
        val normalizedReaderName = normalizeReaderName(readerName)
        val existingSession = readerSessions[normalizedReaderName]
        if (existingSession != null) {
            return existingSession
        }

        Log.i(TAG, "No session for reader $normalizedReaderName, attempting to open new session...")
        val service = seService
        if (service == null || !service.isConnected) {
            Log.e(TAG, "SE Service not connected")
            return null
        }

        val reader = service.readers.firstOrNull { it.name == normalizedReaderName }
        if (reader == null) {
            Log.e(TAG, "Reader $normalizedReaderName not found")
            return null
        }

        return try {
            val newSession = reader.openSession()
            readerSessions[normalizedReaderName] = newSession
            if (readerChannels[normalizedReaderName] == null) {
                readerChannels[normalizedReaderName] = ConcurrentHashMap()
            }
            newSession
        } catch (e: Exception) {
            Log.e(TAG, "Failed to open session for reader $normalizedReaderName", e)
            null
        }
    }

    private fun handleDisconnect(args: JSONObject): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val rawReaderName =
            args.optString("reader").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "reader name required")
        val readerName = normalizeReaderName(rawReaderName)

        val cleanup = cleanupReaderSessions(readerName)
        cleanupFailureResult(cleanup)?.let {
            return it
        }
        return HostCallResult.Success(true)
    }

    private fun handleReset(): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        Log.i(TAG, "Resetting SEService")
        val cleanup = cleanupAllSessions()
        cleanupFailureResult(cleanup)?.let {
            return it
        }
        try {
            seService?.shutdown()
            seService = null
        } catch (e: Exception) {
            val info =
                cleanupCoordinator.markPoisoned(
                    readerName = null,
                    reason = "SEService.shutdown() failed during reset: ${e.omapiReason()}",
                    operationMayHaveSucceeded = true,
                )
            return HostCallResult.Error(
                OMAPI_SESSION_CORRUPTED,
                CORRUPTED_MESSAGE,
                poisonDetails(info),
            )
        }
        if (!initializeSEService(wait = true)) {
            return HostCallResult.Error("NOT_CONNECTED", "SEService could not be initialized")
        }
        return HostCallResult.Success(true)
    }

    private fun handleTransmit(args: JSONObject): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val rawReaderName =
            args.optString("reader").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "reader name required")
        val readerName = normalizeReaderName(rawReaderName)
        val apduHex =
            args.optString("apdu").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "apdu required")

        val session =
            getOrCreateSession(readerName)
                ?: return HostCallResult.Error("NOT_CONNECTED", "No session for reader $readerName")

        return try {
            val apduBytes = apduHex.decodeHexBytes()

            // OMAPI requires logical channel, open with default eUICC AID
            val aidBytes = DEFAULT_EUICC_AID.decodeHexBytes()
            val currentSession = readerSessions[readerName] ?: session
            val channel =
                currentSession.openLogicalChannel(aidBytes)
                    ?: return HostCallResult.Error(
                        "CHANNEL_FAILED",
                        "Failed to open logical channel",
                    )

            // Never overwrite a previously tracked channel. If this newly opened duplicate cannot
            // be closed individually, fail closed instead of losing the old channel reference.
            trackNewChannelOrReject(readerName, DEFAULT_EUICC_AID, channel)?.let {
                return it
            }

            val response = channel.transmit(apduBytes)
            val responseHex = response.toHexStringLower()
            if (profileSwitchInProgress && responseHex.endsWith("6f00", ignoreCase = true)) {
                val info =
                    cleanupCoordinator.markPoisoned(
                        readerName,
                        "6F00 during profile switch",
                        operationMayHaveSucceeded = true,
                    )
                return HostCallResult.Error(
                    OMAPI_SESSION_CORRUPTED,
                    CORRUPTED_MESSAGE,
                    poisonDetails(info),
                )
            }
            try {
                channel.close()
                readerChannels[readerName]?.remove(DEFAULT_EUICC_AID, channel)
            } catch (closeError: Exception) {
                val info =
                    cleanupCoordinator.markPoisoned(
                        readerName,
                        closeError.omapiReason(),
                        operationMayHaveSucceeded = profileSwitchInProgress,
                    )
                return HostCallResult.Error(
                    OMAPI_SESSION_CORRUPTED,
                    CORRUPTED_MESSAGE,
                    poisonDetails(info),
                )
            }
            HostCallResult.Success(responseHex)
        } catch (e: Exception) {
            Log.e(TAG, "Transmit failed", e)
            reportTransmitException(readerName, DEFAULT_EUICC_AID, e)
        }
    }

    private fun handleOpenChannel(args: JSONObject, isRetry: Boolean = false): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val rawReaderName =
            args.optString("reader").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "reader name required")
        val readerName = normalizeReaderName(rawReaderName)

        val singleAid = args.optString("aid").takeIf { it.isNotEmpty() }
        val aidsList = args.optStringList("aids")

        if (singleAid == null && aidsList.isEmpty()) {
            return HostCallResult.Error("INVALID_ARGUMENT", "aid or aids required")
        }

        val session =
            getOrCreateSession(readerName)
                ?: return HostCallResult.Error("NOT_CONNECTED", "No session for reader $readerName")

        val targets = aidsList.ifEmpty { listOf(singleAid!!) }
        var lastError: Exception? = null

        for (targetAid in targets) {
            val targetKey = targetAid.uppercase()
            if (readerChannels[readerName]?.containsKey(targetKey) == true) {
                return HostCallResult.Error(
                    "CHANNEL_ALREADY_OPEN",
                    "A logical channel is already tracked for AID $targetAid",
                )
            }

            try {
                val aidBytes = targetAid.decodeHexBytes()
                val channel = session.openLogicalChannel(aidBytes)

                if (channel != null) {
                    trackNewChannelOrReject(readerName, targetAid, channel)?.let {
                        return it
                    }
                    readersWithSuccessfulChannel.add(readerName)

                    Log.i(TAG, "Opened logical channel for AID: $targetAid on reader: $readerName")

                    val responseMap =
                        mutableMapOf<String, Any>("success" to true, "aid" to targetAid)
                    channel.selectResponse?.let {
                        responseMap["selectResponse"] = it.toHexStringLower()
                    }
                    return HostCallResult.Success(responseMap)
                }
            } catch (e: Exception) {
                lastError = e
                if (e is SecurityException) {
                    Log.w(TAG, "Security exception for AID $targetAid: ${e.message}")
                }
            }
        }

        if (!isRetry && lastError != null) {
            val message = lastError!!.message ?: ""
            val isInfrastructureError =
                message.contains("OpenLogicalChannel() failed", ignoreCase = true) ||
                    lastError is NoSuchElementException

            if (
                isInfrastructureError ||
                    (readersWithSuccessfulChannel.contains(readerName) && message.isNotEmpty())
            ) {
                Log.w(
                    TAG,
                    "Infrastructure error or persistent failure ($message), retrying session...",
                )
                val cleanup = cleanupReaderSessions(readerName)
                cleanupFailureResult(cleanup)?.let {
                    return it
                }
                try {
                    Thread.sleep(150)
                    val reader = seService!!.readers.firstOrNull { it.name == readerName }
                    if (reader != null) {
                        val newSession = reader.openSession()
                        readerSessions[readerName] = newSession
                        readerChannels[readerName] = ConcurrentHashMap()
                        return handleOpenChannel(args, true)
                    }
                } catch (retryEx: Exception) {
                    Log.e(TAG, "Session re-open failed during retry", retryEx)
                }
            }
        }
        val code =
            if (lastError is SecurityException) {
                "AccessControlException"
            } else {
                "CHANNEL_FAILED"
            }
        return HostCallResult.Error(code, lastError?.message ?: "No AID opened")
    }

    private fun handleCloseChannel(args: JSONObject): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val rawReaderName =
            args.optString("reader").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "reader name required")
        val readerName = normalizeReaderName(rawReaderName)
        val aidHex =
            args.optString("aid").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "aid required")

        Log.i(TAG, "Closing tracked channel for AID request: $aidHex")
        val cleanup =
            cleanupCoordinator.cleanupChannel(
                readerName,
                aidHex.uppercase(),
                operationMayHaveSucceeded = profileSwitchInProgress,
            )
        cleanupFailureResult(cleanup)?.let {
            return it
        }
        return HostCallResult.Success(true)
    }

    private fun handleCloseChannels(args: JSONObject): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val rawReaderName =
            args.optString("reader").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "reader name required")
        val readerName = normalizeReaderName(rawReaderName)

        val cleanup = cleanupReaderSessions(readerName)
        cleanupFailureResult(cleanup)?.let {
            return it
        }
        return HostCallResult.Success(true)
    }

    private fun handleTransmitOnChannel(args: JSONObject): HostCallResult {
        enterPoisonedResult()?.let {
            return it
        }
        val rawReaderName =
            args.optString("reader").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "reader name required")
        val readerName = normalizeReaderName(rawReaderName)
        val aidHex =
            args.optString("aid").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "aid required")
        val apduHex =
            args.optString("apdu").takeIf { it.isNotEmpty() }
                ?: return HostCallResult.Error("INVALID_ARGUMENT", "apdu required")

        var attempt = 0
        var currentDelay = 1000L
        val maxAttempts = 8

        while (true) {
            enterPoisonedResult()?.let {
                return it
            }
            val channels = readerChannels[readerName]
            var channel = channels?.get(aidHex.uppercase())

            if (channel == null) {
                if (attempt > 0) {
                    try {
                        val session = getOrCreateSession(readerName)
                        if (session != null) {
                            val aidBytes = aidHex.decodeHexBytes()
                            channel = session.openLogicalChannel(aidBytes)
                            if (channel != null) {
                                trackNewChannelOrReject(readerName, aidHex, channel)?.let {
                                    return it
                                }
                                Log.i(TAG, "Re-opened channel for AID $aidHex during retry $attempt")
                            }
                        }
                    } catch (e: Exception) {
                        Log.e(
                            TAG,
                            "Failed to re-open channel for $aidHex during retry $attempt",
                            e,
                        )
                    }
                } else {
                    val availableAids = channels?.keys?.joinToString(", ") ?: "none"
                    Log.e(
                        TAG,
                        "CHANNEL_NOT_FOUND: Reader=$readerName, AID=${aidHex.uppercase()}. Available: $availableAids",
                    )
                    return HostCallResult.Error("CHANNEL_NOT_FOUND", "No channel for AID: $aidHex")
                }
            }

            if (channel != null) {
                try {
                    val apduBytes = apduHex.decodeHexBytes()
                    Log.i(TAG, "[$readerName] >> $apduHex")
                    val response = channel.transmit(apduBytes)
                    val responseHex = response.toHexStringLower()
                    Log.i(TAG, "[$readerName] << $responseHex")

                    val sw1 = response[response.size - 2]
                    val sw2 = response[response.size - 1]
                    val is6D00 = sw1 == 0x6D.toByte() && sw2 == 0x00.toByte()
                    val is6881 = sw1 == 0x68.toByte() && sw2 == 0x81.toByte()
                    val is6F00 = sw1 == 0x6F.toByte() && sw2 == 0x00.toByte()

                    if (profileSwitchInProgress && (is6F00 || is6D00 || is6881)) {
                        val info =
                            cleanupCoordinator.markPoisoned(
                                readerName,
                                "${responseHex.takeLast(4).uppercase()} during profile switch",
                                operationMayHaveSucceeded = true,
                            )
                        return HostCallResult.Error(
                            OMAPI_SESSION_CORRUPTED,
                            CORRUPTED_MESSAGE,
                            poisonDetails(info),
                        )
                    }

                    if (!is6D00 && !is6881) {
                        readersWithSuccessfulChannel.add(readerName)
                    }

                    if (is6881) {
                        return if (readersWithSuccessfulChannel.contains(readerName)) {
                            Log.w(
                                TAG,
                                "6881 (Proactive Refresh) on reader $readerName. Returning BUSY error to host and cleaning up.",
                            )
                            val cleanup = cleanupReaderSessions(readerName)
                            cleanupFailureResult(cleanup)?.let {
                                return it
                            }
                            HostCallResult.Error(
                                "BUSY",
                                "Card busy - session unavailable",
                                responseHex,
                            )
                        } else {
                            Log.e(
                                TAG,
                                "Got 6881 on reader $readerName with no previous successful APDUs. Likely logical channel not supported.",
                            )
                            HostCallResult.Error(
                                "ERROR",
                                "Card returned 6881 - Logical channel not supported or card busy",
                                responseHex,
                            )
                        }
                    }

                    if (is6D00) {
                        Log.w(
                            TAG,
                            "Got 6D00 on reader $readerName, AID $aidHex. Attempt ${attempt + 1}/$maxAttempts",
                        )

                        if (attempt < maxAttempts) {
                            val cleanup = cleanupReaderSessions(readerName)
                            cleanupFailureResult(cleanup)?.let {
                                return it
                            }

                            Thread.sleep(currentDelay)
                            currentDelay = (currentDelay * 1.5).toLong()
                            attempt++
                            continue
                        }
                    }

                    return HostCallResult.Success(responseHex)
                } catch (e: Exception) {
                    Log.e(TAG, "Channel transmit failed for AID $aidHex (attempt $attempt)", e)
                    return reportTransmitException(readerName, aidHex, e)
                }
            } else if (attempt < maxAttempts) {
                Thread.sleep(currentDelay)
                currentDelay = (currentDelay * 1.5).toLong()
                attempt++
            } else {
                return HostCallResult.Error(
                    "TRANSMIT_FAILED",
                    "Exhausted retries for AID $aidHex",
                )
            }
        }
    }

    private fun cleanupReaderSessions(readerName: String): OmapiCleanupResult {
        val normalizedReaderName = normalizeReaderName(readerName)
        val result = cleanupCoordinator.cleanupReader(normalizedReaderName)
        if (result is OmapiCleanupResult.RebootRequired) {
            Log.e(
                TAG,
                "Channel close failed for $normalizedReaderName; local references were dropped and OMAPI is poisoned",
            )
        }
        return result
    }

    private fun cleanupAllSessions(): OmapiCleanupResult = cleanupCoordinator.cleanupAll()
}
