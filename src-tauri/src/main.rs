// The desktop entry point (M8). Android does not use it: the Gradle project
// `tauri android init` generates loads the `cdylib` and calls `run` through
// `tauri::mobile_entry_point`, which is why the surface lives in `lib.rs`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cabas_tauri_lib::run();
}
