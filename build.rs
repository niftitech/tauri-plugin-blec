const COMMANDS: &[&str] = &[
    "scan",
    "stop_scan",
    "connect",
    "disconnect",
    "connection_state",
    "send",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
