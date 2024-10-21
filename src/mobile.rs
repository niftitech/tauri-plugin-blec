use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::android;

#[cfg(target_os = "android")]
// initializes the Kotlin plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<()> {
    let handle = api.register_android_plugin("com.plugin.blec", "BleClientPlugin")?;
    android::set_app_handle(app.clone());
    Ok(Blec(handle))
}
