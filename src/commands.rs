use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{async_runtime, command, AppHandle, Runtime};
use uuid::Uuid;

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
) -> Result<()> {
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
    handler.lock().await.discover(Some(tx), timeout).await?;
    Ok(())
}

#[command]
pub(crate) async fn stop_scan<R: Runtime>(_app: AppHandle<R>) -> Result<()> {
    tracing::info!("Stopping BLE scan");
    let handler = get_handler()?;
    handler.lock().await.stop_scan().await?;
    Ok(())
}

#[command]
pub(crate) async fn connect<R: Runtime>(
    _app: AppHandle<R>,
    address: String,
    service: Uuid,
    characs: Vec<Uuid>,
    on_disconnect: Channel<()>,
) -> Result<()> {
    tracing::info!("Connecting to BLE device: {:?}", address);
    let mut handler = get_handler()?.lock().await;
    let disconnct_handler = move || {
        on_disconnect
            .send(())
            .expect("failed to send disconnect event to the front-end");
    };
    handler
        .connect(address, service, characs, Some(disconnct_handler))
        .await?;
    Ok(())
}

#[command]
pub(crate) async fn disconnect<R: Runtime>(_app: AppHandle<R>) -> Result<()> {
    tracing::info!("Disconnecting from BLE device");
    let handler = get_handler()?;
    handler.lock().await.disconnect().await?;
    Ok(())
}

pub fn commands<R: Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool {
    tauri::generate_handler![scan, stop_scan, connect, disconnect]
}
