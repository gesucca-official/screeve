// Nella build di rilascio niente finestra della console accanto all'app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod correttore;
mod dati;
mod docx;

use correttore::Refuso;
use std::path::PathBuf;
use tauri::{Emitter, Manager, State};

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

/// I refusi di ogni paragrafo; `None` finché il dizionario non è caricato.
#[tauri::command]
async fn controlla(testi: Vec<String>, stato: State<'_, correttore::Stato>) -> Result<Option<Vec<Vec<Refuso>>>, String> {
    stato.con(|c| testi.iter().map(|t| c.controlla(t)).collect())
}

#[tauri::command]
async fn suggerisci(parola: String, stato: State<'_, correttore::Stato>) -> Result<Vec<String>, String> {
    Ok(stato.con(|c| c.suggerisci(&parola))?.unwrap_or_default())
}

#[tauri::command]
async fn aggiungi_parola(parola: String, stato: State<'_, correttore::Stato>) -> Result<(), String> {
    stato.con_mut(|c| c.aggiungi(&parola))?.unwrap_or(Ok(()))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(correttore::Stato::default())
        .setup(|app| {
            // il dizionario si carica a parte: l'editor è subito utilizzabile
            let app = app.handle().clone();
            std::thread::spawn(move || {
                app.state::<correttore::Stato>().imposta(correttore::Correttore::carica());
                let _ = app.emit("correttore-pronto", ());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            apri_docx,
            salva_docx,
            file_iniziale,
            controlla,
            suggerisci,
            aggiungi_parola
        ])
        .run(tauri::generate_context!())
        .expect("errore all'avvio di Screeve");
}
