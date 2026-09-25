// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    peek_lib::boot_timing::mark_process_start();
    peek_lib::boot_timing::phase("main entry");
    peek_lib::configure_prestart_webview();
    peek_lib::boot_timing::phase("prestart webview configured");
    peek_lib::run()
}
