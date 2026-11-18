package ee.nekoko.nlpa2

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.telephony.IccOpenLogicalChannelResponse
import android.telephony.TelephonyManager
import android.util.Log
import java.lang.reflect.Method
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import org.json.JSONObject

internal class TelephonyPlugin {
    private var context: Context? = null
    private var telephonyManager: TelephonyManager? = null
    private var eventSink: HostEventSink? = null
    private var receiverRegistered = false

    // Dedicated background thread for hardware operations
    private var backgroundHandler: Handler? = null
    private var backgroundThread: HandlerThread? = null

    private val simStateReceiver =
        object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                val action = intent?.action ?: return
                if (
                    action == "android.intent.action.SIM_STATE_CHANGED" ||
                        action == "android.telephony.action.SIM_CARD_STATE_CHANGED" ||
                        action == "android.telephony.action.SIM_APPLICATION_STATE_CHANGED"
                ) {
                    val state = intent.getStringExtra("ss") ?: "UNKNOWN"
                    Log.d(TAG, "SIM state changed event: $action, state: $state")
                    eventSink?.emit(
                        "sim_state_changed",
                        mapOf("state" to state, "action" to action),
                    )
                }
            }
        }

    companion object {
        private const val TAG = "TelephonyPlugin"
        private const val CALL_TIMEOUT_SECONDS = 60L

        // Hidden APIs via reflection
        private val getUiccCardsInfo: Method? by lazy {
            try {
                TelephonyManager::class.java.getMethod("getUiccCardsInfo")
            } catch (_: Exception) {
                null
            }
        }

        private val iccOpenLogicalChannelBySlot: Method? by lazy {
            try {
                TelephonyManager::class.java.getMethod(
                    "iccOpenLogicalChannelBySlot",
                    Int::class.java,
                    String::class.java,
                    Int::class.java,
                )
            } catch (_: Exception) {
                null
            }
        }

        private val iccOpenLogicalChannelByPort: Method? by lazy {
            val tmClass = TelephonyManager::class.java
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    tmClass.getMethod(
                        "iccOpenLogicalChannelByPort",
                        Int::class.java,
                        Int::class.java,
                        String::class.java,
                        Int::class.java,
                    )
                } else {
                    tmClass.getMethod(
                        "iccOpenLogicalChannelByPort",
                        Int::class.java,
                        String::class.java,
                        Int::class.java,
                    )
                }
            } catch (_: Exception) {
                null
            }
        }

        private val iccCloseLogicalChannelBySlot: Method? by lazy {
            try {
                TelephonyManager::class.java.getMethod(
                    "iccCloseLogicalChannelBySlot",
                    Int::class.java,
                    Int::class.java,
                )
            } catch (_: Exception) {
                null
            }
        }

        private val iccCloseLogicalChannelByPort: Method? by lazy {
            val tmClass = TelephonyManager::class.java
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    tmClass.getMethod(
                        "iccCloseLogicalChannelByPort",
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                    )
                } else {
                    tmClass.getMethod(
                        "iccCloseLogicalChannelByPort",
                        Int::class.java,
                        Int::class.java,
                    )
                }
            } catch (_: Exception) {
                null
            }
        }

        private val iccTransmitApduLogicalChannelBySlot: Method? by lazy {
            try {
                TelephonyManager::class.java.getMethod(
                    "iccTransmitApduLogicalChannelBySlot",
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    String::class.java,
                )
            } catch (_: Exception) {
                null
            }
        }

        private val iccTransmitApduLogicalChannelByPort: Method? by lazy {
            val tmClass = TelephonyManager::class.java
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    tmClass.getMethod(
                        "iccTransmitApduLogicalChannelByPort",
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        String::class.java,
                    )
                } else {
                    tmClass.getMethod(
                        "iccTransmitApduLogicalChannelByPort",
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        String::class.java,
                    )
                }
            } catch (_: Exception) {
                null
            }
        }

        private val iccTransmitApduBasicChannelBySlot: Method? by lazy {
            try {
                TelephonyManager::class.java.getMethod(
                    "iccTransmitApduBasicChannelBySlot",
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    Int::class.java,
                    String::class.java,
                )
            } catch (_: Exception) {
                null
            }
        }

        private val iccTransmitApduBasicChannelByPort: Method? by lazy {
            val tmClass = TelephonyManager::class.java
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    tmClass.getMethod(
                        "iccTransmitApduBasicChannelByPort",
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        String::class.java,
                    )
                } else {
                    tmClass.getMethod(
                        "iccTransmitApduBasicChannelByPort",
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        Int::class.java,
                        String::class.java,
                    )
                }
            } catch (_: Exception) {
                null
            }
        }
    }

    fun attach(appContext: Context, sink: HostEventSink) {
        val ctx = appContext.applicationContext
        context = ctx
        eventSink = sink
        telephonyManager = ctx.getSystemService(Context.TELEPHONY_SERVICE) as? TelephonyManager
        startBackgroundThread()

        if (!receiverRegistered) {
            val filter =
                IntentFilter("android.intent.action.SIM_STATE_CHANGED").apply {
                    addAction("android.telephony.action.SIM_CARD_STATE_CHANGED")
                    addAction("android.telephony.action.SIM_APPLICATION_STATE_CHANGED")
                }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                ctx.registerReceiver(simStateReceiver, filter, Context.RECEIVER_EXPORTED)
            } else {
                @Suppress("UnspecifiedRegisterReceiverFlag")
                ctx.registerReceiver(simStateReceiver, filter)
            }
            receiverRegistered = true
        }
    }

    fun detach() {
        val ctx = context
        if (receiverRegistered && ctx != null) {
            try {
                ctx.unregisterReceiver(simStateReceiver)
            } catch (_: Exception) {}
            receiverRegistered = false
        }
        stopBackgroundThread()
        eventSink = null
        telephonyManager = null
        context = null
    }

    private fun startBackgroundThread() {
        if (backgroundThread == null) {
            val thread = HandlerThread("TelephonyPluginBackend").apply { start() }
            backgroundThread = thread
            backgroundHandler = Handler(thread.looper)
        }
    }

    private fun stopBackgroundThread() {
        backgroundThread?.quitSafely()
        backgroundThread = null
        backgroundHandler = null
    }

    fun call(method: String, args: JSONObject): HostCallResult {
        val handler =
            backgroundHandler
                ?: return HostCallResult.Error("ERROR", "Background thread not available")
        val future = CompletableFuture<HostCallResult>()
        val posted =
            handler.post {
                val result =
                    try {
                        when (method) {
                            "hasPrivilegedPermission" -> handleHasPrivilegedPermission()
                            "listSlots" -> handleListSlots()
                            "openChannel" -> handleOpenChannel(args)
                            "closeChannel" -> handleCloseChannel(args)
                            "cleanupAllChannels" -> handleCleanupAllChannels(args)
                            "transmit" -> handleTransmit(args)
                            "sendTerminalCapabilities" -> handleSendTerminalCapabilities(args)
                            else ->
                                HostCallResult.Error(
                                    "UNSUPPORTED_METHOD",
                                    "Unknown Telephony method: $method",
                                )
                        }
                    } catch (e: Exception) {
                        Log.e(TAG, "Error in background method: $method", e)
                        HostCallResult.Error("ERROR", e.message ?: "Telephony error")
                    }
                future.complete(result)
            }
        if (!posted) {
            return HostCallResult.Error("ERROR", "Background thread not available")
        }
        return try {
            future.get(CALL_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        } catch (e: Exception) {
            HostCallResult.Error("ERROR", e.message ?: "Telephony call timed out")
        }
    }

    private fun iccOpenLogicalChannel(
        tm: TelephonyManager,
        slotIndex: Int,
        portIndex: Int,
        aid: String?,
        p2: Int,
    ): IccOpenLogicalChannelResponse {
        return if (iccOpenLogicalChannelByPort != null) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                iccOpenLogicalChannelByPort!!.invoke(tm, slotIndex, portIndex, aid, p2)
                    as IccOpenLogicalChannelResponse
            } else {
                iccOpenLogicalChannelByPort!!.invoke(tm, portIndex, aid, p2)
                    as IccOpenLogicalChannelResponse
            }
        } else if (iccOpenLogicalChannelBySlot != null) {
            iccOpenLogicalChannelBySlot!!.invoke(tm, slotIndex, aid, p2)
                as IccOpenLogicalChannelResponse
        } else {
            throw Exception("Logical channel APIs not available")
        }
    }

    private fun iccCloseLogicalChannel(
        tm: TelephonyManager,
        slotIndex: Int,
        portIndex: Int,
        channel: Int,
    ) {
        if (iccCloseLogicalChannelByPort != null) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                iccCloseLogicalChannelByPort!!.invoke(tm, slotIndex, portIndex, channel)
            } else {
                iccCloseLogicalChannelByPort!!.invoke(tm, portIndex, channel)
            }
        } else if (iccCloseLogicalChannelBySlot != null) {
            iccCloseLogicalChannelBySlot!!.invoke(tm, slotIndex, channel)
        } else {
            throw Exception("Logical channel APIs not available")
        }
    }

    private fun iccTransmitApduLogicalChannel(
        tm: TelephonyManager,
        slotIndex: Int,
        portIndex: Int,
        channel: Int,
        cla: Int,
        ins: Int,
        p1: Int,
        p2: Int,
        p3: Int,
        data: String?,
    ): String? {
        return if (iccTransmitApduLogicalChannelByPort != null) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                iccTransmitApduLogicalChannelByPort!!.invoke(
                    tm,
                    slotIndex,
                    portIndex,
                    channel,
                    cla,
                    ins,
                    p1,
                    p2,
                    p3,
                    data,
                ) as String?
            } else {
                iccTransmitApduLogicalChannelByPort!!.invoke(
                    tm,
                    portIndex,
                    channel,
                    cla,
                    ins,
                    p1,
                    p2,
                    p3,
                    data,
                ) as String?
            }
        } else if (iccTransmitApduLogicalChannelBySlot != null) {
            iccTransmitApduLogicalChannelBySlot!!.invoke(
                tm,
                slotIndex,
                channel,
                cla,
                ins,
                p1,
                p2,
                p3,
                data,
            ) as String?
        } else {
            throw Exception("Logical channel APIs not available")
        }
    }

    private fun iccTransmitApduBasicChannel(
        tm: TelephonyManager,
        slotIndex: Int,
        portIndex: Int,
        cla: Int,
        ins: Int,
        p1: Int,
        p2: Int,
        p3: Int,
        data: String?,
    ): String? {
        return if (iccTransmitApduBasicChannelByPort != null) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                iccTransmitApduBasicChannelByPort!!.invoke(
                    tm,
                    slotIndex,
                    portIndex,
                    cla,
                    ins,
                    p1,
                    p2,
                    p3,
                    data,
                ) as String?
            } else {
                iccTransmitApduBasicChannelByPort!!.invoke(
                    tm,
                    portIndex,
                    cla,
                    ins,
                    p1,
                    p2,
                    p3,
                    data,
                ) as String?
            }
        } else if (iccTransmitApduBasicChannelBySlot != null) {
            iccTransmitApduBasicChannelBySlot!!.invoke(
                tm,
                slotIndex,
                cla,
                ins,
                p1,
                p2,
                p3,
                data,
            ) as String?
        } else {
            throw Exception("Basic channel APIs not available")
        }
    }

    private fun handleHasPrivilegedPermission(): HostCallResult {
        if (telephonyManager == null) {
            return HostCallResult.Error("NOT_AVAILABLE", "TelephonyManager not available")
        }
        val hasPermission =
            try {
                context?.checkSelfPermission("android.permission.MODIFY_PHONE_STATE") ==
                    PackageManager.PERMISSION_GRANTED
            } catch (_: Exception) {
                false
            }
        return HostCallResult.Success(hasPermission)
    }

    private fun handleListSlots(): HostCallResult {
        val tm =
            telephonyManager
                ?: return HostCallResult.Error("NOT_AVAILABLE", "TelephonyManager not available")

        val slots = mutableListOf<Map<String, Any>>()

        if (getUiccCardsInfo != null) {
            try {
                val cardsInfo = getUiccCardsInfo!!.invoke(tm) as? List<*>
                cardsInfo?.forEach { card ->
                    if (card == null) return@forEach
                    val cardMap = mutableMapOf<String, Any>()
                    var slotIndex = -1

                    try {
                        val cardClass = card.javaClass
                        slotIndex =
                            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                                try {
                                    cardClass.getMethod("getPhysicalSlotIndex").invoke(card) as? Int
                                        ?: -1
                                } catch (_: Exception) {
                                    try {
                                        cardClass.getMethod("getSlotIndex").invoke(card) as? Int
                                            ?: -1
                                    } catch (_: Exception) {
                                        -1
                                    }
                                }
                            } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                                try {
                                    cardClass.getMethod("getSlotIndex").invoke(card) as? Int ?: -1
                                } catch (_: Exception) {
                                    -1
                                }
                            } else {
                                -1
                            }

                        if (slotIndex == -1) {
                            Log.d(
                                TAG,
                                "Skipping card with invalid slot index (likely inactive eSIM)",
                            )
                            return@forEach
                        }

                        val isEuiccVal =
                            try {
                                cardClass.getMethod("isEuicc").invoke(card) as? Boolean ?: false
                            } catch (_: Exception) {
                                false
                            }

                        val cardIdVal =
                            try {
                                cardClass.getMethod("getCardId").invoke(card)
                            } catch (_: Exception) {
                                null
                            }

                        val isRemovableVal =
                            try {
                                cardClass.getMethod("isRemovable").invoke(card) as? Boolean ?: true
                            } catch (_: Exception) {
                                true
                            }

                        cardMap["slotIndex"] = slotIndex
                        cardMap["isEuicc"] = isEuiccVal
                        cardMap["cardId"] = cardIdVal?.toString() ?: ""
                        cardMap["isRemovable"] = isRemovableVal
                    } catch (e: Exception) {
                        Log.w(TAG, "Dynamic access failed for card: ${e.message}")
                    }

                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                        try {
                            val ports = card.javaClass.getMethod("getPorts").invoke(card) as? Collection<*>
                            val portList = mutableListOf<Map<String, Any>>()
                            ports?.forEach { port ->
                                if (port == null) return@forEach
                                try {
                                    val portMap = mutableMapOf<String, Any>()
                                    val portClass = port.javaClass
                                    portMap["portIndex"] =
                                        portClass.getMethod("getPortIndex").invoke(port) as? Int
                                            ?: 0
                                    portMap["logicalSlotIndex"] =
                                        portClass.getMethod("getLogicalSlotIndex").invoke(port)
                                            as? Int ?: 0
                                    portList.add(portMap)
                                } catch (e: Exception) {
                                    Log.w(TAG, "Failed to get port info: ${e.message}")
                                }
                            }
                            cardMap["ports"] = portList
                        } catch (_: Exception) {
                            cardMap["ports"] =
                                listOf(mapOf("portIndex" to 0, "logicalSlotIndex" to slotIndex))
                        }
                    } else {
                        cardMap["ports"] =
                            listOf(mapOf("portIndex" to 0, "logicalSlotIndex" to slotIndex))
                    }

                    cardMap["simState"] =
                        try {
                            tm.getSimState(slotIndex)
                        } catch (_: Exception) {
                            0
                        }

                    slots.add(cardMap)
                }
            } catch (e: Exception) {
                Log.w(TAG, "Failed to getUiccCardsInfo: ${e.message}")
            }
        }

        if (slots.isEmpty()) {
            val count = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) tm.activeModemCount else 1
            for (i in 0 until count) {
                val state =
                    try {
                        tm.getSimState(i)
                    } catch (_: Exception) {
                        0
                    }
                slots.add(
                    mapOf(
                        "slotIndex" to i,
                        "isEuicc" to true,
                        "cardId" to "",
                        "isRemovable" to false,
                        "ports" to listOf(mapOf("portIndex" to 0, "logicalSlotIndex" to i)),
                        "simState" to state,
                    ),
                )
            }
        }

        return HostCallResult.Success(slots)
    }

    private fun handleOpenChannel(args: JSONObject): HostCallResult {
        val tm =
            telephonyManager
                ?: return HostCallResult.Error("NOT_AVAILABLE", "TelephonyManager not available")
        val slotIndex = args.optInt("slotIndex", 0)
        val portIndex = args.optInt("portIndex", 0)
        val aid = args.optString("aid", "")

        // SGP.22: Send Terminal Capabilities on basic channel before opening logical channel
        try {
            Log.d(
                TAG,
                "Sending Terminal Capabilities on basic channel (Slot $slotIndex Port $portIndex)...",
            )
            iccTransmitApduBasicChannel(
                tm,
                slotIndex,
                portIndex,
                0x80,
                0xAA,
                0x00,
                0x00,
                0x0A,
                "A9088100820101830107",
            )
        } catch (e: Exception) {
            Log.w(TAG, "Failed to send Terminal Capabilities on BASIC channel: ${e.message}")
        }

        return try {
            val response = iccOpenLogicalChannel(tm, slotIndex, portIndex, aid, 0)
            if (response.status == IccOpenLogicalChannelResponse.STATUS_NO_ERROR) {
                val resultMap = linkedMapOf<String, Any?>("channel" to response.channel)
                response.selectResponse?.let { resultMap["selectResponse"] = it.toHexStringLower() }
                HostCallResult.Success(resultMap)
            } else {
                HostCallResult.Error("OPEN_FAILED", "Status: ${response.status}")
            }
        } catch (e: Exception) {
            val root = e.rootCause()
            HostCallResult.Error("OPEN_FAILED", root.readableMessage())
        }
    }

    private fun handleCloseChannel(args: JSONObject): HostCallResult {
        val tm =
            telephonyManager
                ?: return HostCallResult.Error("NOT_AVAILABLE", "TelephonyManager not available")
        val slotIndex = args.optInt("slotIndex", 0)
        val portIndex = args.optInt("portIndex", 0)
        val channel = args.optInt("channel", -1)

        return try {
            iccCloseLogicalChannel(tm, slotIndex, portIndex, channel)
            HostCallResult.Success(true)
        } catch (e: Exception) {
            val root = e.rootCause()
            HostCallResult.Error("CLOSE_FAILED", root.readableMessage())
        }
    }

    private fun handleCleanupAllChannels(args: JSONObject): HostCallResult {
        val tm =
            telephonyManager
                ?: return HostCallResult.Error("NOT_AVAILABLE", "TelephonyManager not available")
        val slotIndex = args.optInt("slotIndex", 0)
        val portIndex = args.optInt("portIndex", 0)

        return try {
            for (channel in 1..19) {
                try {
                    iccCloseLogicalChannel(tm, slotIndex, portIndex, channel)
                } catch (_: Exception) {
                    // Ignore errors for individual channels (likely not open)
                }
            }
            HostCallResult.Success(true)
        } catch (e: Exception) {
            val root = e.rootCause()
            HostCallResult.Error("CLEANUP_FAILED", root.readableMessage())
        }
    }

    private fun handleTransmit(args: JSONObject): HostCallResult {
        val tm =
            telephonyManager
                ?: return HostCallResult.Error("NOT_AVAILABLE", "TelephonyManager not available")
        val slotIndex = args.optInt("slotIndex", 0)
        val portIndex = args.optInt("portIndex", 0)
        val channel = args.optInt("channel", -1)
        val apdu = args.optString("apdu", "")

        return try {
            val tx = apdu.decodeHexBytes()
            if (tx.size < 4) throw IllegalArgumentException("APDU too short")

            val cla = tx[0].toUByte().toInt()
            val ins = tx[1].toUByte().toInt()
            val p1 = tx[2].toUByte().toInt()
            val p2 = tx[3].toUByte().toInt()
            var p3 = 0
            var data: String? = null

            if (tx.size > 4) {
                p3 = tx[4].toUByte().toInt()
                if (tx.size > 5) {
                    data = tx.sliceArray(5 until tx.size).toHexStringLower()
                }
            }

            val response =
                if (channel == 0) {
                    iccTransmitApduBasicChannel(
                        tm,
                        slotIndex,
                        portIndex,
                        cla,
                        ins,
                        p1,
                        p2,
                        p3,
                        data,
                    )
                } else {
                    iccTransmitApduLogicalChannel(
                        tm,
                        slotIndex,
                        portIndex,
                        channel,
                        cla,
                        ins,
                        p1,
                        p2,
                        p3,
                        data,
                    )
                }

            HostCallResult.Success(response ?: "")
        } catch (e: Exception) {
            val root = e.rootCause()
            HostCallResult.Error("TRANSMIT_FAILED", root.readableMessage())
        }
    }

    private fun handleSendTerminalCapabilities(args: JSONObject): HostCallResult {
        val tm =
            telephonyManager
                ?: return HostCallResult.Error("NOT_AVAILABLE", "TelephonyManager not available")
        val slotIndex = args.optInt("slotIndex", 0)
        val portIndex = args.optInt("portIndex", 0)

        return try {
            val response =
                iccTransmitApduBasicChannel(
                    tm,
                    slotIndex,
                    portIndex,
                    0x80,
                    0xAA,
                    0x00,
                    0x00,
                    0x0A,
                    "A9088100820101830107",
                )
            HostCallResult.Success(response ?: "")
        } catch (e: Exception) {
            val root = e.rootCause()
            HostCallResult.Error("ERROR", root.readableMessage())
        }
    }
}
