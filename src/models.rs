use btleplug::api::{BDAddr, Peripheral};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Eq, Deserialize, Serialize)]
pub struct BleDevice {
    pub address: BleAddress,
    pub name: String,
    pub is_connected: bool,
}

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
    pub async fn from_peripheral(
        peripheral: &btleplug::platform::Peripheral,
    ) -> Result<Self, error::Error> {
        Ok(Self {
            address: peripheral.address().into(),
            name: peripheral
                .properties()
                .await?
                .unwrap_or_default()
                .local_name
                .ok_or(error::Error::UnknownPeripheral(peripheral.id().to_string()))?,
            is_connected: peripheral.is_connected().await?,
        })
    }
}

#[derive(
    Debug, Clone, Copy, Ord, Eq, PartialOrd, PartialEq, Hash, Default, Serialize, Deserialize,
)]
pub struct BleAddress {
    pub address: [u8; 6],
}
impl PartialEq<BDAddr> for BleAddress {
    fn eq(&self, other: &BDAddr) -> bool {
        self.address.eq(&other.into_inner())
    }
}
impl From<BDAddr> for BleAddress {
    fn from(addr: BDAddr) -> Self {
        Self {
            address: addr.into_inner(),
        }
    }
}
impl std::fmt::Display for BleAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let a = &self.address;
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            a[0], a[1], a[2], a[3], a[4], a[5]
        )
    }
}
impl BleAddress {
    // Parses a Bluetooth address with colons `:` as delimiters.
    pub fn from_str_delim(addr_str: &str) -> Result<Self, ParseBleAddressError> {
        match BDAddr::from_str_delim(addr_str) {
            Ok(addr) => Ok(addr.into()),
            Err(_) => Err(ParseBleAddressError),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ParseBleAddressError;
