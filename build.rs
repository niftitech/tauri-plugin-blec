const COMMANDS: &[&str] = &["scan", "stop_scan", "connect", "disconnect"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
