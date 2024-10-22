use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{async_runtime, command, AppHandle, Runtime};

use crate::error::{self, Result};
use crate::get_handler;
use crate::models::BleDevice;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Devices {
    devices: Vec<BleDevice>,
}
#[command]
pub(crate) async fn scan<R: Runtime>(
    _app: AppHandle<R>,
    timeout: u64,
    on_devices: Channel<Devices>,
) -> Result<Vec<BleDevice>> {
    tracing::info!("Scanning for BLE devices");
    let handler = get_handler()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    async_runtime::spawn(async move {
        while let Some(devices) = rx.recv().await {
            on_devices
                .send(Devices { devices })
                .expect("failed to send device to the front-end");
        }
    });
    let devices = handler.discover(Some(tx), timeout).await?;
    Ok(devices)
}

pub fn commands<R: Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool {
    tauri::generate_handler![scan]
}
