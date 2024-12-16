use crate::error::Error;
use crate::models::{fmt_addr, BleDevice, Service};
use btleplug::api::CentralEvent;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[cfg(target_os = "android")]
use crate::android::{Adapter, Manager, Peripheral};
#[cfg(target_os = "android")]
use btleplug::api::{Central as _, Manager as _, Peripheral as _};
#[cfg(not(target_os = "android"))]
use btleplug::platform::{Adapter, Manager, Peripheral};

type ListenerCallback = Arc<dyn Fn(&[u8]) + Send + Sync>;
struct Listener {
    uuid: Uuid,
    callback: ListenerCallback,
}

pub struct Handler {
    connected: Option<Peripheral>,
    characs: Vec<Characteristic>,
    devices: Arc<Mutex<HashMap<String, Peripheral>>>,
    adapter: Arc<Adapter>,
    listen_handle: Option<async_runtime::JoinHandle<()>>,
    notify_listeners: Arc<Mutex<Vec<Listener>>>,
    on_disconnect: Option<Mutex<Box<dyn Fn() + Send>>>,
    connection_update_channel: Option<mpsc::Sender<bool>>,
    scan_update_channel: Option<mpsc::Sender<bool>>,
    scan_task: Option<tokio::task::JoinHandle<()>>,
}

async fn get_central() -> Result<Adapter, Error> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().ok_or(Error::NoAdapters)?;
    Ok(central)
}

impl Handler {
    pub(crate) async fn new() -> Result<Self, Error> {
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
            scan_task: None,
            scan_update_channel: None,
        })
    }

    /// Returns true if a device is connected
    pub fn is_connected(&self) -> bool {
        self.connected.is_some()
    }

    /// Returns true if the adapter is scanning
    pub fn is_scanning(&self) -> bool {
        if let Some(handle) = &self.scan_task {
            !handle.is_finished()
        } else {
            false
        }
    }

    /// Takes a sender that will be used to send changes in the scanning status
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// use tokio::sync::mpsc;
    /// async_runtime::block_on(async {
    ///     let handler = tauri_plugin_blec::get_handler().unwrap();
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///     handler.lock().await.set_scanning_update_channel(tx);
    ///     while let Some(scanning) = rx.recv().await {
    ///         println!("Scanning: {scanning}");
    ///     }
    /// });
    /// ```
    pub fn set_scanning_update_channel(&mut self, tx: mpsc::Sender<bool>) {
        self.scan_update_channel = Some(tx);
    }

    /// Takes a sender that will be used to send changes in the connection status
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// use tokio::sync::mpsc;
    /// async_runtime::block_on(async {
    ///     let handler = tauri_plugin_blec::get_handler().unwrap();
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///     handler.lock().await.set_connection_update_channel(tx);
    ///     while let Some(connected) = rx.recv().await {
    ///         println!("Connected: {connected}");
    ///     }
    /// });
    /// ```
    pub fn set_connection_update_channel(&mut self, tx: mpsc::Sender<bool>) {
        self.connection_update_channel = Some(tx);
    }

    /// Connects to the given address
    /// If a callback is provided, it will be called when the device is disconnected
    /// # Errors
    /// Returns an error if no devices are found, if the device is already connected,
    /// if the connection fails, or if the service/characteristics discovery fails
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// async_runtime::block_on(async {
    ///    let handler = tauri_plugin_blec::get_handler().unwrap();
    ///    handler.lock().await.connect("00:00:00:00:00:00".to_string(),Some(|| println!("disconnected"))).await.unwrap();
    /// });
    /// ```
    pub async fn connect(
        &mut self,
        address: String,
        on_disconnect: Option<Box<dyn Fn() + Send>>,
    ) -> Result<(), Error> {
        if self.devices.lock().await.len() == 0 {
            self.discover(None, 1000, vec![]).await?;
        }
        // connect to the given address
        self.connect_device(address).await?;
        // set callback to run on disconnect
        if let Some(cb) = on_disconnect {
            self.on_disconnect = Some(Mutex::new(cb));
        }
        // discover service/characteristics
        self.connect_services().await?;
        // start background task for notifications
        self.listen_handle = Some(async_runtime::spawn(listen_notify(
            self.connected.clone(),
            self.notify_listeners.clone(),
        )));
        Ok(())
    }

    async fn connect_services(&mut self) -> Result<(), Error> {
        let device = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        device.discover_services().await?;
        let services = device.services();
        for s in services {
            for c in &s.characteristics {
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

    /// Disconnects from the connected device
    /// # Errors
    /// Returns an error if no device is connected or if the disconnect operation fails
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
            tx.send(false).await?;
        }
        self.characs.clear();
        Ok(())
    }

    /// Scans for [timeout] milliseconds and periodically sends discovered devices
    /// to the given channel.
    /// A task is spawned to handle the scan and send the devices, so the function
    /// returns immediately.
    ///
    /// A list of Service UUIDs can be provided to filter the devices discovered.
    /// Devices that provide at least one of the services in the list will be included.
    /// An empty list will include all devices.
    ///
    /// # Errors
    /// Returns an error if starting the scan fails
    /// # Panics
    /// Panics if there is an error getting devices from the adapter
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// use tokio::sync::mpsc;
    /// async_runtime::block_on(async {
    ///     let handler = tauri_plugin_blec::get_handler().unwrap();
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///     handler.lock().await.discover(Some(tx),1000).await.unwrap();
    ///     while let Some(devices) = rx.recv().await {
    ///         println!("Discovered {devices:?}");
    ///     }
    /// });
    /// ```
    pub async fn discover(
        &mut self,
        tx: Option<mpsc::Sender<Vec<BleDevice>>>,
        timeout: u64,
        filter: Vec<Uuid>,
    ) -> Result<(), Error> {
        // stop any ongoing scan
        if let Some(handle) = self.scan_task.take() {
            handle.abort();
            self.adapter.stop_scan().await?;
        }
        // start a new scan
        self.adapter
            .start_scan(ScanFilter { services: filter })
            .await?;
        if let Some(tx) = &self.scan_update_channel {
            tx.send(true).await?;
        }
        let mut self_devices = self.devices.clone();
        let adapter = self.adapter.clone();
        let scan_update_channel = self.scan_update_channel.clone();
        self.scan_task = Some(tokio::task::spawn(async move {
            self_devices.lock().await.clear();
            let loops = timeout / 200;
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
                            .expect("failed to send devices");
                    }
                }
            }
            adapter.stop_scan().await.expect("failed to stop scan");
            if let Some(tx) = &scan_update_channel {
                tx.send(false).await.expect("failed to send scan update");
            }
        }));
        Ok(())
    }

    /// Discover provided services and charecteristics
    /// If the device is not connected, a connection is made in order to discover the services and characteristics
    /// After the discovery is done, the device is disconnected
    /// If the devices was already connected, it will stay connected
    pub async fn discover_services(&self, address: &str) -> Result<Vec<Service>, Error> {
        let mut already_connected = self
            .connected
            .as_ref()
            .is_some_and(|dev| address == fmt_addr(dev.address()));
        let device = if already_connected {
            self.connected.as_ref().expect("Connection exists").clone()
        } else {
            let devices = self.devices.lock().await;
            let device = devices
                .get(address)
                .ok_or(Error::UnknownPeripheral(address.to_string()))?;
            if device.is_connected().await? {
                already_connected = true;
            } else {
                device.connect().await?;
            }
            device.clone()
        };
        if device.services().is_empty() {
            device.discover_services().await?;
        }
        let services = device.services().iter().map(Service::from).collect();
        if !already_connected {
            device.disconnect().await?;
        }
        Ok(services)
    }

    /// Stops scanning for devices
    /// # Errors
    /// Returns an error if stopping the scan fails
    pub async fn stop_scan(&mut self) -> Result<(), Error> {
        self.adapter.stop_scan().await?;
        if let Some(handle) = self.scan_task.take() {
            handle.abort();
        }
        if let Some(tx) = &self.scan_update_channel {
            tx.send(false).await?;
        }
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
                    warn!("Failed to add device: {e}");
                }
            }
        }
        devices.sort();
        devices
    }

    /// Sends data to the given characteristic of the connected device
    /// # Errors
    /// Returns an error if no device is connected or the characteristic is not available
    /// or if the write operation fails
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// use uuid::{Uuid,uuid};
    /// const CHARACTERISTIC_UUID: Uuid = uuid!("51FF12BB-3ED8-46E5-B4F9-D64E2FEC021B");
    /// async_runtime::block_on(async {
    ///     let handler = tauri_plugin_blec::get_handler().unwrap();
    ///     let data = [1,2,3,4,5];
    ///     let response = handler.lock().await.send_data(CHARACTERISTIC_UUID,&data).await.unwrap();
    /// });
    /// ```
    pub async fn send_data(&mut self, c: Uuid, data: &[u8]) -> Result<(), Error> {
        let dev = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = self.get_charac(c)?;
        dev.write(charac, data, WriteType::WithoutResponse).await?;
        Ok(())
    }

    /// Receives data from the given characteristic of the connected device
    /// Returns the data as a vector of bytes
    /// # Errors
    /// Returns an error if no device is connected or the characteristic is not available
    /// or if the read operation fails
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// use uuid::{Uuid,uuid};
    /// const CHARACTERISTIC_UUID: Uuid = uuid!("51FF12BB-3ED8-46E5-B4F9-D64E2FEC021B");
    /// async_runtime::block_on(async {
    ///     let handler = tauri_plugin_blec::get_handler().unwrap();
    ///     let response = handler.lock().await.recv_data(CHARACTERISTIC_UUID).await.unwrap();
    /// });
    /// ```
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

    /// Subscribe to notifications from the given characteristic
    /// The callback will be called whenever a notification is received
    /// # Errors
    /// Returns an error if no device is connected or the characteristic is not available
    /// or if the subscribe operation fails
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// use uuid::{Uuid,uuid};
    /// const CHARACTERISTIC_UUID: Uuid = uuid!("51FF12BB-3ED8-46E5-B4F9-D64E2FEC021B");
    /// async_runtime::block_on(async {
    ///     let handler = tauri_plugin_blec::get_handler().unwrap();
    ///     let response = handler.lock().await.subscribe(CHARACTERISTIC_UUID,|data| println!("received {data:?}")).await.unwrap();
    /// });
    /// ```
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

    /// Unsubscribe from notifications for the given characteristic
    /// This will also remove the callback from the list of listeners
    /// # Errors
    /// Returns an error if no device is connected or the characteristic is not available
    /// or if the unsubscribe operation fails
    pub async fn unsubscribe(&mut self, c: Uuid) -> Result<(), Error> {
        let dev = self.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = self.get_charac(c)?;
        dev.unsubscribe(charac).await?;
        let mut listeners = self.notify_listeners.lock().await;
        listeners.retain(|l| l.uuid != charac.uuid);
        Ok(())
    }

    pub(super) async fn get_event_stream(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = CentralEvent> + Send>>, Error> {
        let events = self.adapter.events().await?;
        Ok(events)
    }

    pub(crate) async fn handle_event(&mut self, event: CentralEvent) -> Result<(), Error> {
        match event {
            CentralEvent::DeviceDisconnected(_) => self.disconnect().await,
            _ => Ok(()),
        }
    }

    /// Returns the connected device
    /// # Errors
    /// Returns an error if no device is connected
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
