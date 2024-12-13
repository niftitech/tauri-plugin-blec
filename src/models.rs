use btleplug::api::BDAddr;
use enumflags2::BitFlags;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingRequest {
    pub value: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BleDevice {
    pub address: String,
    pub name: String,
    pub services: Vec<Service>,
    pub is_connected: bool,
}

impl Eq for BleDevice {}

impl PartialOrd for BleDevice {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BleDevice {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.address.cmp(&other.address)
    }
}

impl PartialEq for BleDevice {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl BleDevice {
    pub async fn from_peripheral<P: btleplug::api::Peripheral>(
        peripheral: &P,
    ) -> Result<Self, error::Error> {
        #[cfg(target_vendor = "apple")]
        let address = peripheral.id().to_string();
        #[cfg(not(target_vendor = "apple"))]
        let address = peripheral.address().to_string();
        let name = peripheral
            .properties()
            .await?
            .unwrap_or_default()
            .local_name
            .unwrap_or_else(|| peripheral.id().to_string());
        let mut services = peripheral.services();
        if services.is_empty() {
            peripheral.discover_services().await?;
            services = peripheral.services();
        }
        let services = services.iter().map(Service::from).collect::<Vec<_>>();
        Ok(Self {
            address,
            name,
            services,
            is_connected: peripheral.is_connected().await?,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Service {
    pub uuid: Uuid,
    pub characteristics: Vec<Characteristic>,
}

impl From<&btleplug::api::Service> for Service {
    fn from(service: &btleplug::api::Service) -> Self {
        Self {
            uuid: service.uuid,
            characteristics: service
                .characteristics
                .iter()
                .map(Characteristic::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Characteristic {
    pub uuid: Uuid,
    pub descriptors: Vec<Uuid>,
    pub properties: BitFlags<CharProps>,
}

impl From<&btleplug::api::Characteristic> for Characteristic {
    fn from(characteristic: &btleplug::api::Characteristic) -> Self {
        Self {
            uuid: characteristic.uuid,
            descriptors: characteristic.descriptors.iter().map(|d| d.uuid).collect(),
            properties: get_flags(characteristic.properties),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[enumflags2::bitflags]
#[repr(u8)]
pub enum CharProps {
    Broadcast,
    Read,
    WriteWithoutResponse,
    Write,
    Notify,
    Indicate,
    AuthenticatedSignedWrites,
    ExtendedProperties,
}

impl From<btleplug::api::CharPropFlags> for CharProps {
    fn from(flag: btleplug::api::CharPropFlags) -> Self {
        match flag {
            btleplug::api::CharPropFlags::BROADCAST => CharProps::Broadcast,
            btleplug::api::CharPropFlags::READ => CharProps::Read,
            btleplug::api::CharPropFlags::WRITE_WITHOUT_RESPONSE => CharProps::WriteWithoutResponse,
            btleplug::api::CharPropFlags::WRITE => CharProps::Write,
            btleplug::api::CharPropFlags::NOTIFY => CharProps::Notify,
            btleplug::api::CharPropFlags::INDICATE => CharProps::Indicate,
            btleplug::api::CharPropFlags::AUTHENTICATED_SIGNED_WRITES => {
                CharProps::AuthenticatedSignedWrites
            }
            btleplug::api::CharPropFlags::EXTENDED_PROPERTIES => CharProps::ExtendedProperties,
            _ => unreachable!(),
        }
    }
}

fn get_flags(properties: btleplug::api::CharPropFlags) -> BitFlags<CharProps, u8> {
    let mut flags = BitFlags::empty();
    for flag in properties.iter() {
        flags |= CharProps::from(flag);
    }
    flags
}

pub fn fmt_addr(addr: BDAddr) -> String {
    let a = addr.into_inner();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        a[0], a[1], a[2], a[3], a[4], a[5]
    )
}
