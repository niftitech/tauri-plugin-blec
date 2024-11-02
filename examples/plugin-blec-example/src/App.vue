<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { BleDevice } from 'tauri-plugin-blec'
import { onMounted, ref } from 'vue';
import BleDev from './components/BleDev.vue'
import { Channel, invoke } from '@tauri-apps/api/core';

const devices = ref<BleDevice[]>([])
const connected = ref(false)

async function get_connection_updates() {
  let connection_chan = new Channel<boolean>()
  connection_chan.onmessage = (state: boolean) => {
    console.log('connection state', connected)
    connected.value = state
  }
  await invoke('plugin:blec|connection_state', { update: connection_chan })
}
onMounted(async () => {
  await get_connection_updates()
})

interface Devices {
  devices: BleDevice[]
}
async function startScan() {
  console.log('start scan')
  devices.value = []
  let onDevices = new Channel<Devices>()
  onDevices.onmessage = (d: Devices) => {
    console.log('onDevices', d.devices.map(d => d.name))
    devices.value = d.devices
  }
  console.log(await invoke<Devices>('plugin:blec|scan', {
    timeout: 5000,
    onDevices
  }))
}

async function stopScan() {
  console.log('stop scan')
  await invoke('plugin:blec|stop_scan')
}

async function disconnect() {
  console.log('disconnect')
  await invoke('plugin:blec|disconnect')
}

const SERVICE_UUID = 'A07498CA-AD5B-474E-940D-16F1FBE7E8CD'
const CHARACTERISTIC_UUID = '51FF12BB-3ED8-46E5-B4F9-D64E2FEC021B'
async function connect(device: BleDevice) {
  console.log('connect', device.address)
  let onDisconnect = new Channel()
  onDisconnect.onmessage = () => {
    console.log(`device ${device.address} disconnected`)
  }
  try {
    await invoke('plugin:blec|connect', {
      address: device.address,
      service: SERVICE_UUID,
      characs: [CHARACTERISTIC_UUID],
      onDisconnect
    })
    devices.value = []
  } catch (e) {
    console.error(e)
    await disconnect()
  }
}

const sendData = ref('')

async function send() {
  await invoke('plugin:blec|send_string', {
    characteristic: CHARACTERISTIC_UUID,
    data: sendData.value
  })
}

const recvData = ref('')

async function read() {
  let res = await invoke<string>('plugin:blec|recv_string', {
    characteristic: CHARACTERISTIC_UUID
  })
  recvData.value = res
}

const notifyData = ref('')
async function subscribe() {
  if (notifyData.value) {
    await invoke('plugin:blec|unsubscribe', {
      characteristic: CHARACTERISTIC_UUID
    })
    notifyData.value = ''
  } else {
    let onData = new Channel<string>()
    onData.onmessage = (data: string) => {
      notifyData.value = data
    }
    await invoke('plugin:blec|subscribe_string', {
      characteristic: CHARACTERISTIC_UUID,
      onData
    })
  }
}
</script>

<template>
  <div class="container">
    <h1>Welcome to the blec plugin!</h1>
    <button :onclick="startScan" style="margin-bottom: 5px;">Start Scan</button>
    <button :onclick="stopScan" style="margin-bottom: 5px;">Stop Scan</button>
    <div v-if="connected">
      <p>Connected</p>
      <button :onclick="disconnect" style="margin-bottom: 5px;">Disconnect</button>
      <div class="row">
        <input v-model="sendData" placeholder="Send data" />
        <button class="ml" :onclick="send">Send</button>
      </div>
      <div class="row">
        <input v-model="recvData" readonly />
        <button class="ml" :onclick="read">Read</button>
      </div>
      <div class="row">
        <input v-model="notifyData" readonly />
        <button class="ml" :onclick="subscribe">{{ notifyData ? "Unsubscribe" : "Subscribe" }}</button>
      </div>
    </div>
    <div v-else v-for="device in devices" class="row">
      <BleDev :key="device.address" :device="device" :onclick="() => connect(device)" />
    </div>
  </div>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
  margin-bottom: 5px;
}

.ml {
  margin-left: 5px;
  min-width: 35%;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}

button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }

  button:active {
    background-color: #0f0f0f69;
  }
}
</style>
