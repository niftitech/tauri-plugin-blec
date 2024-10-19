package com.plugin.blec

import android.util.Log

class BleClient {
    fun pong(value: String): String {
        Log.i("Pong", value)
        return value
    }
}
