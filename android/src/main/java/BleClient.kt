package com.plugin.blec

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.util.Log
import app.tauri.annotation.InvokeArg
import app.tauri.plugin.Invoke

import android.bluetooth.BluetoothAdapter
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanSettings
import android.bluetooth.le.ScanFilter.Builder;
import android.bluetooth.le.ScanResult
import android.content.pm.PackageManager
import android.os.ParcelUuid
import androidx.core.app.ActivityCompat
import app.tauri.plugin.Channel
import app.tauri.plugin.JSObject

class BleDevice{

}

@InvokeArg
class ScanParams {
    val services: ArrayList<String> = ArrayList()
    val onDevice: Channel? = null
}

class BleClient(private val activity: Activity) {
    var scanResults: ArrayList<ScanResult> = ArrayList()
    var scanner: BluetoothLeScanner? = null;

    fun startScan(invoke: Invoke) {
        val args = invoke.parseArgs(ScanParams::class.java)
        println("args:"+args.onDevice.toString())

        if (scanner == null) {
            val bluetoothAdapter = BluetoothAdapter.getDefaultAdapter()
                ?: throw RuntimeException("No bluetooth adapter available.")

            scanner = bluetoothAdapter.bluetoothLeScanner
                ?: throw RuntimeException("No bluetooth scanner available for adapter")
        }
        var filters: ArrayList<ScanFilter?>? = null
        if (args.services.size > 0) {
            filters = ArrayList()
            for (uuid in args.services) {
                filters.add(Builder().setServiceUuid(ParcelUuid.fromString(uuid)).build())
            }
        }
        val settings = ScanSettings.Builder()
            .setCallbackType(ScanSettings.CALLBACK_TYPE_ALL_MATCHES)
            .build()
        if (ActivityCompat.checkSelfPermission(
                activity,
                Manifest.permission.BLUETOOTH_SCAN
            ) != PackageManager.PERMISSION_GRANTED
        ) {
            // TODO: Consider calling
            //    ActivityCompat#requestPermissions
            // here to request the missing permissions, and then overriding
            //   public void onRequestPermissionsResult(int requestCode, String[] permissions,
            //                                          int[] grantResults)
            // to handle the case where the user grants the permission. See the documentation
            // for ActivityCompat#requestPermissions for more details.
            invoke.reject("Missing BLE Permission")
            return
        }
        scanner?.startScan(filters, settings, object: ScanCallback(){
            override fun onScanResult(callbackType: Int, result: ScanResult){
                println(result.toString())
                var dev = JSObject()
                dev.put("device",result)
                args.onDevice?.send(dev)
            }
        })

        invoke.resolve()
    }

    @SuppressLint("MissingPermission")
    fun stopScan(invoke: Invoke){
        scanner?.stopScan(object: ScanCallback(){})
        invoke.resolve()
    }
}
