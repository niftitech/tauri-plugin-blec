package com.plugin.blec


import Peripheral
import android.app.Activity
import android.bluetooth.BluetoothDevice
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin


@InvokeArg
class ConnectParams{
    val address: String = ""
}

@TauriPlugin
class BleClientPlugin(private val activity: Activity): Plugin(activity) {
    public var devices: MutableMap<String, Peripheral> = mutableMapOf();
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
}
