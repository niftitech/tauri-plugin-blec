use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Blec;
#[cfg(mobile)]
use mobile::Blec;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the blec APIs.
pub trait BlecExt<R: Runtime> {
    fn blec(&self) -> &Blec<R>;
}

impl<R: Runtime, T: Manager<R>> crate::BlecExt<R> for T {
    fn blec(&self) -> &Blec<R> {
        self.state::<Blec<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("blec")
        .invoke_handler(tauri::generate_handler![commands::ping])
        .setup(|app, api| {
            #[cfg(mobile)]
            let blec = mobile::init(app, api)?;
            #[cfg(desktop)]
            let blec = desktop::init(app, api)?;
            app.manage(blec);
            Ok(())
        })
        .build()
}
