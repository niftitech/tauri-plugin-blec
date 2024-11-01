package com.plugin.blec


import Peripheral
import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.UUID


@InvokeArg
class ConnectParams{
    val address: String = ""
}

@TauriPlugin
class BleClientPlugin(private val activity: Activity): Plugin(activity) {
    var devices: MutableMap<String, Peripheral> = mutableMapOf();
    private val client = BleClient(activity,this)

    @Command
    fun start_scan(invoke: Invoke) {
        client.startScan(invoke)
    }

    @Command
    fun stop_scan(invoke: Invoke){
        client.stopScan(invoke)
    }

    @Command
    fun connect(invoke: Invoke){
        val args = invoke.parseArgs(ConnectParams::class.java)
        val device = this.devices[args.address]
        if (device == null){
            invoke.reject("Device not found")
            return
        }
        device.connect(invoke)
    }

    @Command
    fun disconnect(invoke: Invoke){
        val args = invoke.parseArgs(ConnectParams::class.java)
        val device = this.devices[args.address]
        if (device == null){
            invoke.reject("Device not found")
            return
        }
        device.disconnect(invoke)
    }

    @Command
    fun is_connected(invoke: Invoke){
        val args = invoke.parseArgs(ConnectParams::class.java)
        val device = this.devices[args.address]
        val res = JSObject()
        if (device == null){
            res.put("result", false)
        } else {
            res.put("result",device.isConnected())
        }
        invoke.resolve(res)
    }

    @Command
    fun discover_services(invoke:Invoke){
        val args = invoke.parseArgs(ConnectParams::class.java)
        val device = this.devices[args.address]
        if (device == null){
            invoke.reject("Device not found")
            return
        }
        device.discoverServices(invoke)
    }

    @Command
    fun services(invoke:Invoke){
        val args = invoke.parseArgs(ConnectParams::class.java)
        val device = this.devices[args.address]
        if (device == null){
            invoke.reject("Device not found")
            return
        }
        device.services(invoke)
    }

    class NotifyParams () {
        var address: String = ""
        var channel: Channel? = null
    }

    @Command
    fun subscribe(invoke:Invoke){
        val args = invoke.parseArgs(NotifyParams::class.java)
        val device = this.devices[args.address]
        if (device == null){
            invoke.reject("Device not found")
            return
        }
        device.setNotifyChannel(args.channel!!)
        invoke.resolve()
    }

    class SendParams() {
        val address: String = ""
        val characteristic: UUID? = null
        val data: ByteArray? = null
        val withResponse: Boolean = true
    }
    @Command
    fun write(invoke:Invoke){
        val args = invoke.parseArgs(SendParams::class.java)
        val device = this.devices[args.address]
        if (device == null){
            invoke.reject("Device not found")
            return
        }
        device.write(invoke)
    }
}
