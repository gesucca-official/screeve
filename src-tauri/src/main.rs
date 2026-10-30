// Nella build di rilascio niente finestra della console accanto all'app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod docx;

use std::path::PathBuf;

#[tauri::command]
async fn apri_docx(percorso: String) -> Result<docx::Documento, String> {
    docx::apri(&PathBuf::from(percorso))
}

#[tauri::command]
async fn salva_docx(percorso: String, blocchi: Vec<docx::Blocco>) -> Result<(), String> {
    docx::salva(&PathBuf::from(percorso), &blocchi)
}

/// Il file passato da riga di comando: "Apri con", doppio clic su un .docx associato
/// o trascinamento sull'eseguibile.
#[tauri::command]
fn file_iniziale() -> Option<String> {
    let percorso = PathBuf::from(std::env::args_os().nth(1)?);
    let percorso = std::path::absolute(&percorso).unwrap_or(percorso);
    percorso.is_file().then(|| percorso.to_string_lossy().into_owned())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![apri_docx, salva_docx, file_iniziale])
        .run(tauri::generate_context!())
        .expect("errore all'avvio di Screeve");
}
