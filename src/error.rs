use serde::{Serialize, Serializer};
use tokio::sync::mpsc::error::SendError;

use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Btleplug error: {0}")]
    Btleplug(#[from] btleplug::Error),

    #[error("Call init() first.")]
    RuntimeNotInitialized,

    #[allow(dead_code)]
    #[error("Cannot initialize CLASS_LOADER")]
    ClassLoader,

    #[allow(dead_code)]
    #[error("Cannot initialize RUNTIME")]
    Runtime,

    #[allow(dead_code)]
    #[error("Java vm not initialized")]
    JavaVM,

    #[error("There is no peripheral with id: {0}")]
    UnknownPeripheral(String),

    #[error("Characteristic with uuid {0:?} not found")]
    CharacNotFound(Uuid),

    #[error("Characteristic {0} not available")]
    CharacNotAvailable(String),

    #[error("No device connected")]
    NoDeviceConnected,

    #[error("Service not found")]
    ServiceNotFound,

    #[error("Device is already connected.")]
    AlreadyConnected,

    #[error("Handler not initialized")]
    HandlerNotInitialized,

    #[error("Handler already initialized")]
    HandlerAlreadyInitialized,

    #[error("received wrong data")]
    WrongData,

    #[error("could not send devices: {0}")]
    SendingDevices(SendError<Vec<crate::BleDevice>>),

    #[error("could not join fuure: {0}")]
    JoinError(tokio::task::JoinError),

    #[error("no bluetooth adapters found")]
    NoAdapters,

    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
