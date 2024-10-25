const COMMANDS: &[&str] = &["scan", "stop_scan"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
