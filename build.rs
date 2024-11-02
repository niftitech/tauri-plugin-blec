const COMMANDS: &[&str] = &[
    "scan",
    "stop_scan",
    "connect",
    "disconnect",
    "connection_state",
    "send",
    "send_string",
    "recv",
    "recv_string",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
