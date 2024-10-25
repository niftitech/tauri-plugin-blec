package com.plugin.blec

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.util.Log
import app.tauri.annotation.InvokeArg
import app.tauri.plugin.Invoke

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanSettings
import android.bluetooth.le.ScanFilter.Builder;
import android.bluetooth.le.ScanResult
import android.content.Context.MODE_PRIVATE
import android.content.Intent
import android.content.SharedPreferences
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.ParcelUuid
import android.provider.Settings
import android.widget.Toast
import androidx.core.app.ActivityCompat
import androidx.core.app.ActivityCompat.startActivityForResult
import androidx.core.content.ContextCompat.getSystemService
import app.tauri.plugin.Channel
import app.tauri.plugin.JSObject

class BleDevice(
    val address: String
){
    fun toJsObject():JSObject{
        var obj = JSObject()
        obj.put("address",address)
        return obj
    }
}

@InvokeArg
class ScanParams {
    val services: ArrayList<String> = ArrayList()
    val onDevice: Channel? = null
}

class BleClient(private val activity: Activity) {
    private var scanner: BluetoothLeScanner? = null;
    private var scanCb: ScanCallback? = null;

    private fun markFirstPermissionRequest(perm: String) {
        val sharedPreference: SharedPreferences =
            activity.getSharedPreferences("PREFS_PERMISSION_FIRST_TIME_ASKING", MODE_PRIVATE)
        sharedPreference.edit().putBoolean(perm, false).apply()
    }

    private fun firstPermissionRequest(perm: String): Boolean {
        return activity.getSharedPreferences("PREFS_PERMISSION_FIRST_TIME_ASKING", MODE_PRIVATE)
            .getBoolean(perm, true)
    }

    private fun checkPermissions(): Boolean {

        for (perm in if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            arrayOf(
                Manifest.permission.BLUETOOTH_SCAN,
                Manifest.permission.BLUETOOTH_CONNECT
            )
        } else {
            arrayOf(
                Manifest.permission.BLUETOOTH_ADMIN,
                Manifest.permission.BLUETOOTH,
            )
        }) {
            if (ActivityCompat.checkSelfPermission(
                    activity,
                    perm
                ) != PackageManager.PERMISSION_GRANTED
            ) {
                if (firstPermissionRequest(perm) || activity.shouldShowRequestPermissionRationale(perm)) {
                    // this will open the permission dialog
                    markFirstPermissionRequest(perm)
                    activity.requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 1)
                } else{
                    // this will open settings which asks for permission
                    val intent = Intent(
                        Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                        Uri.parse("package:${activity.packageName}")
                    )
                    activity.startActivity(intent)
                    Toast.makeText(activity, "Allow Permission: $perm", Toast.LENGTH_SHORT).show()
                    return false
                }
            }
        }
        return true
    }

    @SuppressLint("MissingPermission")
    fun startScan(invoke: Invoke) {
        // check if running
        if (scanCb != null){
            invoke.reject("Scan already running")
            return
        }
        // check permission
        if (!checkPermissions()){
            invoke.reject("Missing permissions");
            return
        }

        // get scanner
        if (scanner == null) {
            val bluetoothManager: BluetoothManager = getSystemService(activity, BluetoothManager::class.java)
                ?: throw RuntimeException("No bluetooth manager found")
            val bluetoothAdapter: BluetoothAdapter = bluetoothManager.getAdapter()
                ?: throw RuntimeException("No bluetooth adapter available")
            // check if bluetooth is on
            if (!bluetoothAdapter.isEnabled ) {
                val enableBtIntent = Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE)
                startActivityForResult(activity, enableBtIntent,0,null)
            }
            scanner = bluetoothAdapter.bluetoothLeScanner
                ?: throw RuntimeException("No bluetooth scanner available for adapter")
        }

        val args = invoke.parseArgs(ScanParams::class.java)
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

        scanCb = object: ScanCallback(){
            private fun sendResult(result: ScanResult){
                val device = BleDevice(
                    result.device.address
                )
                val res = JSObject()
                res.put("result", device.toJsObject())
                args.onDevice!!.send(res)
            }
            override fun onBatchScanResults(results: List<ScanResult>){
                for(result in results){
                    sendResult(result)
                }
            }
            override fun onScanFailed(errorCode: Int){
                println("Scan failed with error code $errorCode")
            }
            override fun onScanResult(callbackType: Int, result: ScanResult){
                sendResult(result)
            }
        }
        scanner?.startScan(filters, settings, scanCb!!)

        invoke.resolve()
    }

    @SuppressLint("MissingPermission")
    fun stopScan(invoke: Invoke){
        println("stopScan")
        if (scanCb!=null) {
            scanner?.stopScan(scanCb!!)
            scanCb = null
        }
        invoke.resolve()
    }
}
