import { Channel, invoke } from '@tauri-apps/api/core'

export type BleDevice = {
  address: string;
  name: string;
  isConnected: boolean;
};

type Result<T> = {
  value?: T;
  error?: string;
}

export async function scan(timeout: Number, onDevicesHandler: (devices: BleDevice[]) => void): Promise<Result<BleDevice[]>> {
  const onDevices = new Channel<BleDevice[]>();
  onDevices.onmessage = onDevicesHandler;
  return await invoke<BleDevice[]>('plugin:blec|scan', {
    timeout,
    onDevices
  }).then((res) => { return { value: res }; }).catch((err) => { return { error: err }; });
}
