import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattService
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import org.json.JSONArray


class Peripheral(private val activity: Activity, private val device: BluetoothDevice) {
    private var connected = false
    private var gatt: BluetoothGatt? = null
    private var services: List<BluetoothGattService> = listOf()
    private var onConnectionStateChange: ((connected:Boolean,error:String)->Unit)? = null
    private var onServicesDiscovered: ((connected:Boolean,error:String)->Unit)? = null
    private val callback = object:BluetoothGattCallback(){
        override fun onConnectionStateChange(gatt: BluetoothGatt?, status: Int, newState: Int) {
            if(status == BluetoothGatt.GATT_SUCCESS && newState==BluetoothGatt.STATE_CONNECTED && gatt!=null){
                this@Peripheral.connected = true
                this@Peripheral.gatt = gatt
                this@Peripheral.onConnectionStateChange?.invoke(true,"")
            } else {
                this@Peripheral.connected = false
                this@Peripheral.gatt = null
                this@Peripheral.onConnectionStateChange?.invoke(false,"Not connected. Status: $status, State: $newState")
            }
        }
        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                this@Peripheral.services = listOf()
                this@Peripheral.onServicesDiscovered?.invoke(false,"No services discovered. Status $status")
            } else {
                this@Peripheral.services = gatt.services
                this@Peripheral.onServicesDiscovered?.invoke(true,"")
            }
        }
    }

    @SuppressLint("MissingPermission")
    fun connect(invoke:Invoke) {
        this.onConnectionStateChange = { success, error ->
            if(success){
                invoke.resolve()
            } else {
                invoke.reject(error)
            }
            this@Peripheral.onConnectionStateChange = null
        }
        this.device.connectGatt(activity, false, this.callback)
    }

    @SuppressLint("MissingPermission")
    fun discoverServices(invoke:Invoke){
        if (this.gatt == null){
            invoke.reject("No gatt server connected")
            return
        }
        this.onServicesDiscovered={ success, error ->
            if (success) {
                invoke.resolve()
            } else {
                invoke.reject(error)
            }
            this@Peripheral.onServicesDiscovered = null

        }
        this.gatt!!.discoverServices()
    }

    fun isConnected():Boolean {
        return this.connected
    }

    @SuppressLint("MissingPermission")
    fun disconnect(invoke: Invoke){
        this.gatt?.disconnect()
        this.connected = false
        invoke.resolve()
    }

     class ResCharacteristic (
        uuid: String,
        properties: Int,
        descriptors: List<String>,
     )

    class ResService (
        uuid: String,
        primary: Boolean,
        characs: List<ResCharacteristic>,
    )

    fun services(invoke:Invoke){
        //TODO: return discovered services
        var services = JSONArray()
        for(service in this.services){

        }
        var res = JSObject()
        res.put("result",services)
        invoke.resolve(res)
    }
}
