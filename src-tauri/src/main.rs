// Nella build di rilascio niente finestra della console accanto all'app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("errore all'avvio di Screeve");
}
