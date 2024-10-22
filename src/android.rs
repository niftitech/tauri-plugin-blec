use std::{collections::BTreeSet, pin::Pin, vec};

use async_trait::async_trait;
use btleplug::{
    api::{
        BDAddr, CentralEvent, CentralState, Characteristic, Descriptor, PeripheralProperties,
        Service, ValueNotification, WriteType,
    },
    platform::PeripheralId,
};
use futures::{stream::Once, Stream};
use once_cell::sync::OnceCell;
use tauri::{
    ipc::{Channel, InvokeResponseBody},
    plugin::PluginHandle,
    AppHandle, Manager as _, Wry,
};
use uuid::Uuid;

type Result<T> = std::result::Result<T, btleplug::Error>;

static HANDLE: OnceCell<PluginHandle<Wry>> = OnceCell::new();

fn get_handle() -> &'static PluginHandle<Wry> {
    HANDLE.get().expect("plugin handle not initialized")
}

pub fn init<C: serde::de::DeserializeOwned>(
    app: &AppHandle<Wry>,
    api: tauri::plugin::PluginApi<Wry, C>,
) -> std::result::Result<(), crate::error::Error> {
    let handle = api.register_android_plugin("com.plugin.blec", "BleClientPlugin")?;
    HANDLE.set(handle).unwrap();
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Adapter;

fn on_device_callback(response: InvokeResponseBody) -> std::result::Result<(), tauri::Error> {
    tracing::info!("on_device_callback: {:?}", response);
    Ok(())
}

#[async_trait]
impl btleplug::api::Central for Adapter {
    type Peripheral = Peripheral;

    async fn events(&self) -> Result<Pin<Box<dyn Stream<Item = CentralEvent> + Send>>> {
        todo!()
    }

    async fn start_scan(&self, filter: btleplug::api::ScanFilter) -> Result<()> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct ScanParams {
            services: Vec<Uuid>,
            on_device: Channel<serde_json::Value>,
        }
        let on_device = Channel::new(on_device_callback);
        get_handle()
            .run_mobile_plugin(
                "start_scan",
                ScanParams {
                    services: filter.services,
                    on_device,
                },
            )
            .map_err(|e| btleplug::Error::RuntimeError(e.to_string()))?;
        Ok(())
    }

    async fn stop_scan(&self) -> Result<()> {
        get_handle()
            .run_mobile_plugin("stop_scan", serde_json::Value::Null)
            .map_err(|e| btleplug::Error::RuntimeError(e.to_string()))?;
        Ok(())
    }

    async fn peripherals(&self) -> Result<Vec<Self::Peripheral>> {
        // TODO: Implement this
        Ok(vec![])
    }

    async fn peripheral(&self, id: &PeripheralId) -> Result<Self::Peripheral> {
        todo!()
    }

    async fn add_peripheral(&self, address: &PeripheralId) -> Result<Self::Peripheral> {
        todo!()
    }

    async fn adapter_info(&self) -> Result<String> {
        todo!()
    }

    async fn adapter_state(&self) -> Result<CentralState> {
        todo!()
    }
}

pub struct Manager;

impl Manager {
    pub async fn new() -> Result<Self> {
        Ok(Manager)
    }
}

#[async_trait]
impl btleplug::api::Manager for Manager {
    type Adapter = Adapter;

    async fn adapters(&self) -> Result<Vec<Adapter>> {
        Ok(vec![Adapter])
    }
}

#[derive(Debug, Clone)]
pub struct Peripheral;

#[async_trait::async_trait]
impl btleplug::api::Peripheral for Peripheral {
    fn id(&self) -> PeripheralId {
        todo!()
    }

    fn address(&self) -> BDAddr {
        todo!()
    }

    async fn properties(&self) -> Result<Option<PeripheralProperties>> {
        todo!()
    }

    fn services(&self) -> BTreeSet<Service> {
        todo!()
    }

    async fn is_connected(&self) -> Result<bool> {
        todo!()
    }

    async fn connect(&self) -> Result<()> {
        todo!()
    }

    async fn disconnect(&self) -> Result<()> {
        todo!()
    }

    async fn discover_services(&self) -> Result<()> {
        todo!()
    }

    async fn write(
        &self,
        characteristic: &Characteristic,
        data: &[u8],
        write_type: WriteType,
    ) -> Result<()> {
        todo!()
    }

    async fn read(&self, characteristic: &Characteristic) -> Result<Vec<u8>> {
        todo!()
    }

    async fn subscribe(&self, characteristic: &Characteristic) -> Result<()> {
        todo!()
    }

    async fn unsubscribe(&self, characteristic: &Characteristic) -> Result<()> {
        todo!()
    }

    async fn notifications(&self) -> Result<Pin<Box<dyn Stream<Item = ValueNotification> + Send>>> {
        todo!()
    }

    async fn write_descriptor(&self, descriptor: &Descriptor, data: &[u8]) -> Result<()> {
        todo!()
    }

    async fn read_descriptor(&self, descriptor: &Descriptor) -> Result<Vec<u8>> {
        todo!()
    }
}
