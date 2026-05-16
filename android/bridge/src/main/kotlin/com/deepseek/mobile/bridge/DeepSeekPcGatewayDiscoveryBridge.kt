package com.deepseek.mobile.bridge

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import java.net.InetAddress
import java.nio.charset.Charset
import java.util.ArrayDeque
import java.util.UUID

/**
 * Android NSD/mDNS adapter for DeepSeek PC Host discovery.
 *
 * This bridge only discovers local services and reports records to Rust/Dioxus.
 * It does not execute PC commands and does not trust discovered hosts by itself.
 * Rust core validates candidates, probes /health, applies route scoring, and keeps
 * approval policy in control of any later workspace action.
 */
class DeepSeekPcGatewayDiscoveryBridge(
    context: Context,
    private val callback: Callback
) {
    interface Callback {
        fun onPcGatewayDiscoveryStarted(requestId: String, serviceType: String)
        fun onPcGatewayDiscoveryCandidate(requestId: String, record: AndroidPcGatewayMdnsRecordPayload)
        fun onPcGatewayDiscoveryCompleted(requestId: String, records: List<AndroidPcGatewayMdnsRecordPayload>)
        fun onPcGatewayDiscoveryFailed(requestId: String, message: String)
    }

    private val appContext = context.applicationContext
    private val nsdManager = appContext.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val wifiManager = appContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager
    private var multicastLock: WifiManager.MulticastLock? = null
    private var activeCommand: AndroidPcGatewayDiscoveryCommandPayload? = null
    private var discoveryListener: NsdManager.DiscoveryListener? = null
    private val pendingResolve = ArrayDeque<NsdServiceInfo>()
    private var resolving = false
    private val records = linkedMapOf<String, AndroidPcGatewayMdnsRecordPayload>()

    @Synchronized
    fun startDiscovery(command: AndroidPcGatewayDiscoveryCommandPayload = AndroidPcGatewayDiscoveryCommandPayload()) {
        if (activeCommand != null) {
            stopDiscovery()
        }
        activeCommand = command
        records.clear()
        pendingResolve.clear()
        resolving = false
        acquireMulticastLock()
        val listener = buildDiscoveryListener(command)
        discoveryListener = listener
        callback.onPcGatewayDiscoveryStarted(command.requestId, command.serviceType)
        try {
            nsdManager.discoverServices(command.serviceType, NsdManager.PROTOCOL_DNS_SD, listener)
        } catch (error: Throwable) {
            fail(command.requestId, error.message ?: error.javaClass.simpleName)
        }
    }

    @Synchronized
    fun stopDiscovery() {
        val command = activeCommand
        val listener = discoveryListener
        if (listener != null) {
            try {
                nsdManager.stopServiceDiscovery(listener)
            } catch (_: Throwable) {
                // Android may throw when discovery already stopped; completion still proceeds.
            }
        }
        releaseMulticastLock()
        discoveryListener = null
        activeCommand = null
        pendingResolve.clear()
        resolving = false
        if (command != null) {
            callback.onPcGatewayDiscoveryCompleted(command.requestId, records.values.toList())
        }
    }

    private fun buildDiscoveryListener(command: AndroidPcGatewayDiscoveryCommandPayload): NsdManager.DiscoveryListener {
        return object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(serviceType: String) {
                callback.onPcGatewayDiscoveryStarted(command.requestId, serviceType)
            }

            override fun onServiceFound(serviceInfo: NsdServiceInfo) {
                if (serviceInfo.serviceType != command.serviceType) {
                    return
                }
                enqueueResolve(serviceInfo)
            }

            override fun onServiceLost(serviceInfo: NsdServiceInfo) {
                // Lost events are intentionally not destructive. Rust route scoring will stop
                // preferring an endpoint after failed health checks.
            }

            override fun onDiscoveryStopped(serviceType: String) {
                releaseMulticastLock()
            }

            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                fail(command.requestId, "NSD discovery failed to start for $serviceType: $errorCode")
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                fail(command.requestId, "NSD discovery failed to stop for $serviceType: $errorCode")
            }
        }
    }

    @Synchronized
    private fun enqueueResolve(serviceInfo: NsdServiceInfo) {
        pendingResolve.add(serviceInfo)
        drainResolveQueue()
    }

    @Synchronized
    private fun drainResolveQueue() {
        if (resolving) return
        val command = activeCommand ?: return
        val next = pendingResolve.poll() ?: return
        resolving = true
        try {
            nsdManager.resolveService(next, object : NsdManager.ResolveListener {
                override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                    finishResolve()
                }

                override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                    val record = serviceInfo.toPayload() ?: run {
                        finishResolve()
                        return
                    }
                    synchronized(this@DeepSeekPcGatewayDiscoveryBridge) {
                        records[record.stableKey()] = record
                    }
                    callback.onPcGatewayDiscoveryCandidate(command.requestId, record)
                    finishResolve()
                }
            })
        } catch (_: Throwable) {
            finishResolve()
        }
    }

    @Synchronized
    private fun finishResolve() {
        resolving = false
        drainResolveQueue()
    }

    private fun NsdServiceInfo.toPayload(): AndroidPcGatewayMdnsRecordPayload? {
        val address: InetAddress = host ?: return null
        val resolvedPort = port
        if (resolvedPort <= 0) return null
        val txtMap = attributes.mapValues { entry -> entry.value.toString(Charset.forName("UTF-8")) }
        return AndroidPcGatewayMdnsRecordPayload(
            instanceName = serviceName ?: "DeepSeek PC Host",
            host = address.hostAddress ?: return null,
            port = resolvedPort,
            txt = txtMap
        )
    }

    private fun fail(requestId: String, message: String) {
        releaseMulticastLock()
        discoveryListener = null
        activeCommand = null
        pendingResolve.clear()
        resolving = false
        callback.onPcGatewayDiscoveryFailed(requestId, message)
    }

    private fun acquireMulticastLock() {
        if (multicastLock?.isHeld == true) return
        multicastLock = wifiManager?.createMulticastLock("deepseek-pc-gateway-discovery")?.apply {
            setReferenceCounted(false)
            try {
                acquire()
            } catch (_: Throwable) {
                // Discovery can still work on some networks without the lock; this is non-fatal.
            }
        }
    }

    private fun releaseMulticastLock() {
        val lock = multicastLock
        if (lock?.isHeld == true) {
            try {
                lock.release()
            } catch (_: Throwable) {
                // Non-fatal cleanup path.
            }
        }
        multicastLock = null
    }
}

data class AndroidPcGatewayDiscoveryCommandPayload(
    val requestId: String = UUID.randomUUID().toString(),
    val serviceType: String = "_deepseek-pc-gateway._tcp.",
    val timeoutMs: Long = 5000L
)

data class AndroidPcGatewayMdnsRecordPayload(
    val instanceName: String,
    val host: String,
    val port: Int,
    val txt: Map<String, String> = emptyMap()
) {
    fun stableKey(): String = "$host:$port:$instanceName"
}
