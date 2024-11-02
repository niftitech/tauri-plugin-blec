<script setup lang="ts">
import { BleDevice } from 'tauri-plugin-blec'

defineProps<{
    device: BleDevice
}>()
</script>

<template>
    <div class="box">
        <h1>{{ device.name }}</h1>
        <p>{{ device.address }}</p>
        <p>{{ device.isConnected ? "Connected" : "Not Connected" }}</p>
    </div>
</template>

<style scoped>
.box {
    margin: 0;
    text-align: center;
    width: 80vh;
    margin: 10px;
    padding: 10px;
    box-shadow: 0 0 10px rgba(0, 0, 0, 0.3);
    border-radius: 10px;
    background-color: #f6f6f6;
    color: #0f0f0f;
}

.box:hover {
    background-color: #e6e6e6;
}
</style>
