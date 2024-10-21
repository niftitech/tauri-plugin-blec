type Result<T> = std::result::Result<T, btleplug::Error>;
use std::{collections::BTreeSet, pin::Pin};

use async_trait::async_trait;
use btleplug::{
    api::{
        BDAddr, CentralEvent, CentralState, Characteristic, Descriptor, PeripheralProperties,
        ScanFilter, Service, ValueNotification, WriteType,
    },
    platform::PeripheralId,
};
use futures::Stream;

#[derive(Debug, Clone)]
pub struct Adapter;

#[async_trait]
impl btleplug::api::Central for Adapter {
    type Peripheral = Peripheral;

    async fn events(&self) -> Result<Pin<Box<dyn Stream<Item = CentralEvent> + Send>>> {
        todo!()
    }

    async fn start_scan(&self, filter: ScanFilter) -> Result<()> {
        todo!()
    }

    async fn stop_scan(&self) -> Result<()> {
        todo!()
    }

    async fn peripherals(&self) -> Result<Vec<Self::Peripheral>> {
        todo!()
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
        todo!()
    }
}

#[async_trait]
impl btleplug::api::Manager for Manager {
    type Adapter = Adapter;

    async fn adapters(&self) -> Result<Vec<Adapter>> {
        todo!()
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
