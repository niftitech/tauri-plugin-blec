use crate::error::Error;
use crate::models::{fmt_addr, BleDevice, Service};
use btleplug::api::CentralEvent;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::PeripheralId;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime;
use tokio::sync::{mpsc, watch, Mutex};
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

struct HandlerState {
    connected: Option<Peripheral>,
    characs: Vec<Characteristic>,
    listen_handle: Option<async_runtime::JoinHandle<()>>,
    on_disconnect: Option<Mutex<Box<dyn Fn() + Send>>>,
    connection_update_channel: Option<mpsc::Sender<bool>>,
    scan_update_channel: Option<mpsc::Sender<bool>>,
    scan_task: Option<tokio::task::JoinHandle<()>>,
}

impl HandlerState {
    fn get_charac(&self, uuid: Uuid) -> Result<&Characteristic, Error> {
        let charac = self.characs.iter().find(|c| c.uuid == uuid);
        charac.ok_or(Error::CharacNotAvailable(uuid.to_string()))
    }
}

pub struct Handler {
    devices: Arc<Mutex<HashMap<String, Peripheral>>>,
    adapter: Arc<Adapter>,
    notify_listeners: Arc<Mutex<Vec<Listener>>>,
    connected_rx: watch::Receiver<bool>,
    connected_tx: watch::Sender<bool>,
    state: Mutex<HandlerState>,
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
        let (connected_tx, connected_rx) = watch::channel(false);
        Ok(Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            adapter: Arc::new(central),
            notify_listeners: Arc::new(Mutex::new(vec![])),
            connected_rx,
            connected_tx,
            state: Mutex::new(HandlerState {
                on_disconnect: None,
                connection_update_channel: None,
                scan_task: None,
                scan_update_channel: None,
                listen_handle: None,
                characs: vec![],
                connected: None,
            }),
        })
    }

    /// Returns true if a device is connected
    pub fn is_connected(&self) -> bool {
        *self.connected_rx.borrow()
    }

    /// Returns true if the adapter is scanning
    pub async fn is_scanning(&self) -> bool {
        if let Some(handle) = &self.state.lock().await.scan_task {
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
    ///     handler.set_scanning_update_channel(tx).await;
    ///     while let Some(scanning) = rx.recv().await {
    ///         println!("Scanning: {scanning}");
    ///     }
    /// });
    /// ```
    pub async fn set_scanning_update_channel(&self, tx: mpsc::Sender<bool>) {
        self.state.lock().await.scan_update_channel = Some(tx);
    }

    /// Takes a sender that will be used to send changes in the connection status
    /// # Example
    /// ```no_run
    /// use tauri::async_runtime;
    /// use tokio::sync::mpsc;
    /// async_runtime::block_on(async {
    ///     let handler = tauri_plugin_blec::get_handler().unwrap();
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///     handler.set_connection_update_channel(tx).await;
    ///     while let Some(connected) = rx.recv().await {
    ///         println!("Connected: {connected}");
    ///     }
    /// });
    /// ```
    pub async fn set_connection_update_channel(&self, tx: mpsc::Sender<bool>) {
        self.state.lock().await.connection_update_channel = Some(tx);
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
        &self,
        address: &str,
        on_disconnect: Option<Box<dyn Fn() + Send>>,
    ) -> Result<(), Error> {
        if self.devices.lock().await.len() == 0 {
            self.discover(None, 1000, vec![]).await?;
        }
        // connect to the given address
        self.connect_device(address).await?;
        let mut state = self.state.lock().await;
        // set callback to run on disconnect
        if let Some(cb) = on_disconnect {
            state.on_disconnect = Some(Mutex::new(cb));
        }
        // discover service/characteristics
        self.connect_services(&mut state).await?;
        // start background task for notifications
        state.listen_handle = Some(async_runtime::spawn(listen_notify(
            state.connected.clone(),
            self.notify_listeners.clone(),
        )));
        Ok(())
    }

    async fn connect_services(&self, state: &mut HandlerState) -> Result<(), Error> {
        let device = state.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let mut services = device.services();
        if services.is_empty() {
            device.discover_services().await?;
            services = device.services();
        }
        for s in services {
            for c in &s.characteristics {
                state.characs.push(c.clone());
            }
        }
        Ok(())
    }

    async fn connect_device(&self, address: &str) -> Result<(), Error> {
        debug!("connecting to {address}",);
        let mut state = self.state.lock().await;
        if let Some(dev) = state.connected.as_ref() {
            if address == fmt_addr(dev.address()) {
                return Err(Error::AlreadyConnected);
            }
        }
        let mut connected_rx = self.connected_rx.clone();
        let devices = self.devices.lock().await;
        let device = devices
            .get(address)
            .ok_or(Error::UnknownPeripheral(address.to_string()))?;
        state.connected = Some(device.clone());
        if !device.is_connected().await? {
            assert!(
                !(*connected_rx.borrow_and_update()),
                "connected_rx is true without device being connected, this is a bug"
            );
            debug!("Connecting to device");
            device.connect().await?;
            debug!("Connecting done");
        }
        // wait for the actual connection to be established
        connected_rx
            .changed()
            .await
            .expect("failed to wait for connection event");
        if !*self.connected_rx.borrow() {
            // still not connected
            return Err(Error::ConnectionFailed);
        }

        if let Some(tx) = &state.connection_update_channel {
            tx.send(true)
                .await
                .expect("failed to send connection update");
        }
        Ok(())
    }

    /// Disconnects from the connected device
    /// This triggers a disconnect and then waits for the actual disconnect event from the adapter
    /// # Errors
    /// Returns an error if no device is connected or if the disconnect fails
    /// # Panics
    /// panics if there is an error with handling the internal disconnect event
    pub async fn disconnect(&self) -> Result<(), Error> {
        debug!("disconnect triggered by user");
        let mut connected_rx = self.connected_rx.clone();
        if let Some(dev) = self.state.lock().await.connected.as_mut() {
            if let Ok(true) = dev.is_connected().await {
                assert!(
                    !(*connected_rx.borrow_and_update()),
                    "connected_rx is false with a device being connected, this is a bug"
                );
                dev.disconnect().await?;
            } else {
                debug!("device is not connected");
                return Err(Error::NoDeviceConnected);
            }
        } else {
            debug!("no device connected");
            return Err(Error::NoDeviceConnected);
        }
        debug!("waiting for disconnect event");
        // the change will be triggered by handle_event -> handle_disconnect which runs in another
        // task
        self.connected_rx
            .clone()
            .changed()
            .await
            .expect("failed to wait for disconnect event");
        if *self.connected_rx.borrow() {
            // still connected
            return Err(Error::DisconnectFailed);
        }
        Ok(())
    }

    /// Clears internal state, updates connected flag and calls disconnect callback
    async fn handle_disconnect(&self, peripheral_id: PeripheralId) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        if !state
            .connected
            .as_ref()
            .is_some_and(|dev| dev.id() == peripheral_id)
        {
            // event not for currently connected device, ignore
            return Ok(());
        }
        info!("disconnecting");
        state.connected = None;
        if let Some(handle) = state.listen_handle.take() {
            handle.abort();
        }
        *self.notify_listeners.lock().await = vec![];
        if let Some(on_disconnect) = &state.on_disconnect {
            let callback = on_disconnect.lock().await;
            callback();
        }
        if let Some(tx) = &state.connection_update_channel {
            tx.send(false).await?;
        }
        state.characs.clear();
        self.connected_tx
            .send(false)
            .expect("failed to send connected update");
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
        &self,
        tx: Option<mpsc::Sender<Vec<BleDevice>>>,
        timeout: u64,
        filter: Vec<Uuid>,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        // stop any ongoing scan
        if let Some(handle) = state.scan_task.take() {
            handle.abort();
            self.adapter.stop_scan().await?;
        }
        // start a new scan
        self.adapter
            .start_scan(ScanFilter { services: filter })
            .await?;
        if let Some(tx) = &state.scan_update_channel {
            tx.send(true).await?;
        }
        let mut self_devices = self.devices.clone();
        let adapter = self.adapter.clone();
        let scan_update_channel = state.scan_update_channel.clone();
        state.scan_task = Some(tokio::task::spawn(async move {
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
    /// # Errors
    /// Returns an error if the device is not found, if the connection fails, or if the discovery fails
    /// # Panics
    /// Panics if there is an error with the internal disconnect event
    pub async fn discover_services(&self, address: &str) -> Result<Vec<Service>, Error> {
        let state = self.state.lock().await;
        let mut already_connected = state
            .connected
            .as_ref()
            .is_some_and(|dev| address == fmt_addr(dev.address()));
        let device = if already_connected {
            state.connected.as_ref().expect("Connection exists").clone()
        } else {
            let devices = self.devices.lock().await;
            let device = devices
                .get(address)
                .ok_or(Error::UnknownPeripheral(address.to_string()))?;
            if device.is_connected().await? {
                already_connected = true;
            } else {
                self.connect_device(address).await?;
            }
            device.clone()
        };
        if device.services().is_empty() {
            device.discover_services().await?;
        }
        let services = device.services().iter().map(Service::from).collect();
        if !already_connected {
            let mut connected_rx = self.connected_rx.clone();
            if *connected_rx.borrow_and_update() {
                device.disconnect().await?;
                connected_rx
                    .changed()
                    .await
                    .expect("failed to wait for disconnect event");
            }
        }
        Ok(services)
    }

    /// Stops scanning for devices
    /// # Errors
    /// Returns an error if stopping the scan fails
    pub async fn stop_scan(&self) -> Result<(), Error> {
        self.adapter.stop_scan().await?;
        let mut state = self.state.lock().await;
        if let Some(handle) = state.scan_task.take() {
            handle.abort();
        }
        if let Some(tx) = &state.scan_update_channel {
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
    pub async fn send_data(&self, c: Uuid, data: &[u8]) -> Result<(), Error> {
        let state = self.state.lock().await;
        let dev = state.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = state.get_charac(c)?;
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
    pub async fn recv_data(&self, c: Uuid) -> Result<Vec<u8>, Error> {
        let state = self.state.lock().await;
        let dev = state.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = state.get_charac(c)?;
        let data = dev.read(charac).await?;
        Ok(data)
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
        &self,
        c: Uuid,
        callback: impl Fn(&[u8]) + Send + Sync + 'static,
    ) -> Result<(), Error> {
        let state = self.state.lock().await;
        let dev = state.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = state.get_charac(c)?;
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
    pub async fn unsubscribe(&self, c: Uuid) -> Result<(), Error> {
        let state = self.state.lock().await;
        let dev = state.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let charac = state.get_charac(c)?;
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

    pub(crate) async fn handle_event(&self, event: CentralEvent) -> Result<(), Error> {
        dbg!(&event);
        match event {
            CentralEvent::DeviceDisconnected(peripheral_id) => {
                self.handle_disconnect(peripheral_id).await?;
            }
            CentralEvent::DeviceConnected(peripheral_id) => {
                self.handle_connect(peripheral_id).await;
            }

            _event => {}
        }
        Ok(())
    }

    /// Returns the connected device
    /// # Errors
    /// Returns an error if no device is connected
    pub async fn connected_device(&self) -> Result<BleDevice, Error> {
        let state = self.state.lock().await;
        let p = state.connected.as_ref().ok_or(Error::NoDeviceConnected)?;
        let d = BleDevice::from_peripheral(p).await?;
        Ok(d)
    }

    async fn handle_connect(&self, peripheral_id: PeripheralId) {
        if !self
            .state
            .lock()
            .await
            .connected
            .as_ref()
            .is_some_and(|dev| dev.id() == peripheral_id)
        {
            // event not for currently connected device, ignore
            return;
        }
        info!("\n################################\nconnection to {peripheral_id} established\n#################################################");
        self.connected_tx
            .send(true)
            .expect("failed to send connected update");
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
