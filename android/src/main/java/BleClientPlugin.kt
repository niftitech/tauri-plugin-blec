package com.plugin.blec

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke

@InvokeArg
class ScanFilter {
    val services: List<String>? = null
}

@TauriPlugin
class BleClientPlugin(private val activity: Activity): Plugin(activity) {
    private val implementation = BleClient()

    @Command
    fun start_scan(invoke: Invoke) {
        val args = invoke.parseArgs(ScanFilter::class.java)
        println("Services:" + args.services.toString())

        val ret = JSObject()
        ret.put("status", "ok")
        invoke.resolve(ret)
    }
}
