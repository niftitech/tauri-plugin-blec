import { Channel, invoke } from '@tauri-apps/api/core'

type BleDevice = {
  address: string;
  name: string;
  isConnected: boolean;
};

export async function scan(timeout: Number, onDevicesHandler: (devices: BleDevice[]) => void): Promise<string | null> {
  const onDevices = new Channel<BleDevice[]>();
  onDevices.onmessage = onDevicesHandler;
  return await invoke<{ value?: string }>('plugin:blec|scan', {
    payload: {
      timeout,
      onDevices
    },
  }).then((r) => (r.value ? r.value : null));
}
