package com.plugin.blec


import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin


@TauriPlugin
class BleClientPlugin(private val activity: Activity): Plugin(activity) {
    private val client = BleClient(activity)

    @Command
    fun start_scan(invoke: Invoke) {
        client.startScan(invoke)
    }

    @Command
    fun stop_scan(invoke: Invoke){
        client.stopScan(invoke)
    }
}
