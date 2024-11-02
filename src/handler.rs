use crate::error::Error;
use crate::models::{fmt_addr, BleDevice};
use btleplug::api::CentralEvent;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use futures::{FutureExt, Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[cfg(target_os = "android")]
use crate::android::{Adapter, Manager, Peripheral};
#[cfg(target_os = "android")]
use btleplug::api::{Central as _, Manager as _, Peripheral as _};
#[cfg(not(target_os = "android"))]
use btleplug::platform::{Adapter, Manager, Peripheral};

struct Listener {
    uuid: Uuid,
    callback: Arc<dyn Fn(&[u8]) + Send + Sync>,
}

pub struct BleHandler {
    connected: Option<Peripheral>,
    characs: Vec<Characteristic>,
    devices: Arc<Mutex<HashMap<String, Peripheral>>>,
    adapter: Arc<Adapter>,
    listen_handle: Option<async_runtime::JoinHandle<()>>,
    notify_listeners: Arc<Mutex<Vec<Listener>>>,
    on_disconnect: Option<Mutex<Box<dyn Fn() + Send>>>,
    connection_update_channel: Option<mpsc::Sender<bool>>,
}

async fn get_central() -> Result<Adapter, Error> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().ok_or(Error::NoAdapters)?;
    Ok(central)
}

impl BleHandler {
    pub async fn new() -> Result<Self, Error> {
        let central = get_central().await?;
        Ok(Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            characs: vec![],
            connected: None,
            adapter: Arc::new(central),
            listen_handle: None,
            notify_listeners: Arc::new(Mutex::new(vec![])),
            on_disconnect: None,
            connection_update_channel: None,
        })
    }

    pub async fn is_connected(&self) -> bool {
        self.connected.is_some()
    }

    pub fn set_connection_update_channel(&mut self, tx: mpsc::Sender<bool>) {
        self.connection_update_channel = Some(tx);
    }

    pub async fn connect(
        &mut self,
        address: String,
        service: Uuid,
        characs: Vec<Uuid>,
        on_disconnect: Option<impl Fn() + Send + 'static>,
    ) -> Result<(), Error> {
        if self.devices.lock().await.len() == 0 {
            self.discover(None, 1000).await?;
        }
        // connect to the given address
        self.connect_device(address).await?;
        // set callback to run on disconnect
        if let Some(cb) = on_disconnect {
            self.on_disconnect = Some(Mutex::new(Box::new(cb)));
        }
        // discover service/characteristics
        self.connect_service(service, &characs).await?;
        // start background task for notifications
        self.listen_handle = Some(async_runtime::spawn(listen_notify(
            self.connected.clone(),
            self.notify_listeners.clone(),
        )));
        Ok(())
    }

    async fn connect_service(&mut self, service: Uuid, characs: &[Uuid]) -> Result<(), Error> {
        let device = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        device.discover_services().await?;
        let services = device.services();
        let s = services
            .iter()
            .find(|s| s.uuid == service)
            .ok_or(Error::ServiceNotFound)?;
        for c in &s.characteristics {
            if characs.contains(&c.uuid) {
                self.characs.push(c.clone());
            }
        }
        Ok(())
    }

    async fn connect_device(&mut self, address: String) -> Result<(), Error> {
        debug!("connecting to {address}",);
        if let Some(dev) = self.connected.as_ref() {
            if address == fmt_addr(dev.address()) {
                return Err(Error::AlreadyConnected);
            }
        }
        let devices = self.devices.lock().await;
        let device = devices
            .get(&address)
            .ok_or(Error::UnknownPeripheral(address.to_string()))?;
        if !device.is_connected().await? {
            debug!("Connecting to device");
            device.connect().await?;
            debug!("Connecting done");
        }
        self.connected = Some(device.clone());
        if let Some(tx) = &self.connection_update_channel {
            tx.send(true)
                .await
                .expect("failed to send connection update");
        }
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Error> {
        debug!("disconnecting");
        if let Some(handle) = self.listen_handle.take() {
            handle.abort();
        }
        *self.notify_listeners.lock().await = vec![];
        if let Some(dev) = self.connected.as_mut() {
            if let Ok(true) = dev.is_connected().await {
                dev.disconnect().await?;
            }
            self.connected = None;
        }
        if let Some(on_disconnect) = &self.on_disconnect {
            let callback = on_disconnect.lock().await;
            callback();
        }
        if let Some(tx) = &self.connection_update_channel {
            tx.send(false)
                .await
                .expect("failed to send connection update");
        }
        self.characs.clear();
        Ok(())
    }

    /// Scans for [timeout] milliseconds and periodically sends discovered devices
    /// Also returns vector with all devices after timeout
    pub async fn discover(
        &self,
        tx: Option<mpsc::Sender<Vec<BleDevice>>>,
        timeout: u64,
    ) -> Result<(), Error> {
        self.adapter
            .start_scan(ScanFilter {
                // services: vec![*SERVICE_UUID],
                services: vec![],
            })
            .await?;
        let mut self_devices = self.devices.clone();
        let mut connected = self.connected.clone();
        let adapter = self.adapter.clone();
        tokio::task::spawn(async move {
            self_devices.lock().await.clear();
            let loops = (timeout as f64 / 200.0).round() as u64;
            let mut devices;
            for _ in 0..loops {
                sleep(Duration::from_millis(200)).await;
                let discovered = adapter
                    .peripherals()
                    .await
                    .expect("failed to get peripherals");
                devices = Self::add_devices(&mut self_devices, discovered).await;
                if !devices.is_empty() {
                    if let Some(tx) = &tx {
                        tx.send(devices.clone())
                            .await
                            .map_err(|e| Error::SendingDevices(e))
                            .expect("failed to send devices");
                    }
                }
            }
            adapter.stop_scan().await.expect("failed to stop scan");
        });
        Ok(())
    }

    /// Stops scanning for devices
    pub async fn stop_scan(&self) -> Result<(), Error> {
        self.adapter.stop_scan().await?;
        Ok(())
    }

    async fn add_devices(
        self_devices: &mut Arc<Mutex<HashMap<String, Peripheral>>>,
        discovered: Vec<Peripheral>,
    ) -> Vec<BleDevice> {
        let mut devices = vec![];
        for p in discovered {
            match BleDevice::from_peripheral(&p).await {
                Ok(dev) => {
                    self_devices.lock().await.insert(dev.address.clone(), p);
                    devices.push(dev);
                }
                Err(e) => {
                    error!("Failed to add device: {e}");
                }
            }
        }
        devices.sort();
        devices
    }

    pub async fn send_data(&mut self, c: Uuid, data: &[u8]) -> Result<(), Error> {
        let dev = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = self.get_charac(c)?;
        dev.write(charac, &data, WriteType::WithoutResponse).await?;
        Ok(())
    }

    pub async fn recv_data(&mut self, c: Uuid) -> Result<Vec<u8>, Error> {
        let dev = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = self.get_charac(c)?;
        let data = dev.read(charac).await?;
        Ok(data)
    }

    fn get_charac(&self, uuid: Uuid) -> Result<&Characteristic, Error> {
        let charac = self.characs.iter().find(|c| c.uuid == uuid);
        charac.ok_or(Error::CharacNotAvailable(uuid.to_string()))
    }

    pub async fn subscribe(
        &mut self,
        c: Uuid,
        callback: impl Fn(&[u8]) + Send + Sync + 'static,
    ) -> Result<(), Error> {
        let dev = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = self.get_charac(c)?;
        dev.subscribe(charac).await?;
        self.notify_listeners.lock().await.push(Listener {
            uuid: charac.uuid,
            callback: Arc::new(callback),
        });
        Ok(())
    }

    pub(super) async fn get_event_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = CentralEvent> + Send>>, Error> {
        let events = self.adapter.events().await?;
        Ok(events)
    }

    pub async fn handle_event(&mut self, event: CentralEvent) -> Result<(), Error> {
        // logi!("handling event {event:?}");
        match event {
            CentralEvent::DeviceDisconnected(_) => self.disconnect().await,
            _ => Ok(()),
        }
    }

    pub async fn connected_device(&self) -> Result<BleDevice, Error> {
        let p = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let d = BleDevice::from_peripheral(p).await?;
        Ok(d)
    }
}

async fn listen_notify(dev: Option<Peripheral>, listeners: Arc<Mutex<Vec<Listener>>>) {
    let mut stream = dev
        .expect("no device connected")
        .notifications()
        .await
        .expect("failed to get notifications stream");
    while let Some(data) = stream.next().await {
        info!(
            "listen_notify: data.uuid: {:?}, listener:{}",
            data.uuid,
            listeners.lock().await.len()
        );
        for l in listeners.lock().await.iter() {
            info!("listener.uuid: {:?}", l.uuid);
            if l.uuid == data.uuid {
                let data = data.value.clone();
                let cb = l.callback.clone();
                async_runtime::spawn_blocking(move || cb(&data));
            }
        }
    }
}
