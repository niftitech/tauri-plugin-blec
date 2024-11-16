use btleplug::api::BDAddr;
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
    pub address: String,
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
    pub async fn from_peripheral<P: btleplug::api::Peripheral>(
        peripheral: &P,
    ) -> Result<Self, error::Error> {
        #[cfg(target_vendor = "apple")]
        let address = peripheral.id().to_string();
        #[cfg(not(target_vendor = "apple"))]
        let address = peripheral.address().to_string();
        Ok(Self {
            address,
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

pub fn fmt_addr(addr: BDAddr) -> String {
    let a = addr.into_inner();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        a[0], a[1], a[2], a[3], a[4], a[5]
    )
}
