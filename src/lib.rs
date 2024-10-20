use handler::BleHandler;
use once_cell::sync::OnceCell;
use tauri::{
    async_runtime,
    plugin::{Builder, TauriPlugin},
    Runtime,
};

pub use models::*;

mod commands;
mod error;
mod handler;
mod models;

static HANDLER: OnceCell<BleHandler> = OnceCell::new();
/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    HANDLER
        .set(async_runtime::block_on(BleHandler::new()).expect("failed to initialize handler"))
        .ok()
        .expect("handler already initialized");

    Builder::new("blec")
        .invoke_handler(commands::commands())
        .build()
}

pub fn get_handler() -> error::Result<&'static BleHandler> {
    let handler = HANDLER.get().ok_or(error::Error::HandlerNotInitialized)?;
    Ok(handler)
}
