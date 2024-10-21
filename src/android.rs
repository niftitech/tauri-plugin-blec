type Result<T> = std::result::Result<T, crate::error::Error>;

pub struct Adapter;

pub struct Manager;

pub struct Peripheral;

impl Peripheral {
    pub async fn is_connected(&self) -> Result<bool> {
        todo!()
    }

    pub async fn discover_services(&self) -> Result<()> {
        todo!()
    }
}

pub fn get_central() -> Result<Adapter> {
    Ok(Adapter)
}
