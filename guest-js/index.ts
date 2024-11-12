import { Channel, invoke } from '@tauri-apps/api/core'

export type BleDevice = {
  address: string;
  name: string;
  isConnected: boolean;
};

export async function startScan(handler: (devices: BleDevice[]) => void, timeout: Number) {
  if (!timeout) {
    timeout = 10000;
  }
  let onDevices = new Channel<BleDevice[]>();
  onDevices.onmessage = handler;
  await invoke<BleDevice[]>('plugin:blec|scan', {
    timeout,
    onDevices
  })
}

export async function stopScan() {
  console.log('stop scan')
  await invoke('plugin:blec|stop_scan')
}

export async function getConnectionUpdates(handler: (connected: boolean) => void) {
  let connection_chan = new Channel<boolean>()
  connection_chan.onmessage = handler
  await invoke('plugin:blec|connection_state', { update: connection_chan })
}

export async function disconnect() {
  await invoke('plugin:blec|disconnect')
}

export async function connect(device: BleDevice, onDisconnect: () => void | undefined) {
  console.log('connect', device.address)
  let disconnectChannel = new Channel()
  if (onDisconnect) {
    disconnectChannel.onmessage = onDisconnect
  }
  try {
    await invoke('plugin:blec|connect', {
      address: device.address,
      onDisconnect: disconnectChannel
    })
  } catch (e) {
    console.error(e)
    await disconnect()
  }
}

export async function send(characteristic: string, data: Uint8Array) {
  await invoke('plugin:blec|send', {
    characteristic,
    data
  })
}

export async function sendString(characteristic: string, data: string) {
  await invoke('plugin:blec|send_string', {
    characteristic,
    data
  })
}

export async function read(characteristic: string): Promise<Uint8Array> {
  let res = await invoke<Uint8Array>('plugin:blec|recv', {
    characteristic
  })
  return res
}

export async function readString(characteristic: string): Promise<string> {
  let res = await invoke<string>('plugin:blec|recv_string', {
    characteristic
  })
  return res
}

export async function unsubscribe(characteristic: string) {
  await invoke('plugin:blec|unsubscribe', {
    characteristic
  })
}

export async function subscribe(characteristic: string, handler: (data: Uint8Array) => void) {
  let onData = new Channel<Uint8Array>()
  onData.onmessage = handler;
  await invoke('plugin:blec|subscribe', {
    characteristic,
    onData
  })
}

export async function subscribeString(characteristic: string, handler: (data: string) => void) {
  let onData = new Channel<string>()
  onData.onmessage = handler;
  await invoke('plugin:blec|subscribe_string', {
    characteristic,
    onData
  })
}
