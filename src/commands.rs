use tauri::ipc::Channel;
use tauri::{async_runtime, command, AppHandle, Runtime};

use crate::error::Result;
use crate::get_handler;
use crate::models::BleDevice;

#[command]
pub(crate) async fn scan<R: Runtime>(
    _app: AppHandle<R>,
    timeout: u64,
    on_devices: Channel<Vec<BleDevice>>,
) -> Result<()> {
    let handler = get_handler()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    async_runtime::spawn(async move {
        while let Some(device) = rx.recv().await {
            on_devices
                .send(device)
                .expect("failed to send device to the front-end");
        }
    });
    handler.discover(Some(tx), timeout).await?;
    Ok(())
}

pub fn commands<R: Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool {
    tauri::generate_handler![scan]
}
