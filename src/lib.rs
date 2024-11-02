use std::collections::btree_map::Keys;

use futures::StreamExt;
use handler::BleHandler;
use once_cell::sync::OnceCell;
use tauri::{
    async_runtime,
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, Wry,
};
use tokio::sync::Mutex;

#[cfg(target_os = "android")]
mod android;
mod commands;
mod error;
mod handler;
mod models;

static HANDLER: OnceCell<Mutex<BleHandler>> = OnceCell::new();

/// Initializes the plugin.
pub fn init() -> TauriPlugin<Wry> {
    let handler = async_runtime::block_on(BleHandler::new()).expect("failed to initialize handler");
    let _ = HANDLER.set(Mutex::new(handler));

    Builder::new("blec")
        .invoke_handler(commands::commands())
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            android::init(app, api)?;
            async_runtime::spawn(handle_events());
            Ok(())
        })
        .build()
}

pub fn get_handler() -> error::Result<&'static Mutex<BleHandler>> {
    let handler = HANDLER.get().ok_or(error::Error::HandlerNotInitialized)?;
    Ok(handler)
}

async fn handle_events() {
    let stream = get_handler()
        .expect("failed to get handler")
        .lock()
        .await
        .get_event_stream()
        .await
        .expect("failed to get event stream");
    stream
        .for_each(|event| async {
            get_handler()
                .expect("failed to get handler")
                .lock()
                .await
                .handle_event(event)
                .await
                .expect("failed to handle event");
        })
        .await;
}
