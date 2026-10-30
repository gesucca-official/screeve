// Nella build di rilascio niente finestra della console accanto all'app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod caricato;
mod correttore;
mod dati;
mod docx;
mod recupero;
mod sinonimi;

use caricato::Caricato;
use correttore::{Correttore, Refuso};
use recupero::{Copia, Orfana, Sessione};
use sinonimi::{Sinonimi, Voce};
use std::path::PathBuf;
use tauri::{Emitter, Manager, RunEvent, State};

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
async fn controlla(testi: Vec<String>, stato: State<'_, Caricato<Correttore>>) -> Result<Option<Vec<Vec<Refuso>>>, String> {
    stato.con(|c| testi.iter().map(|t| c.controlla(t)).collect())
}

#[tauri::command]
async fn suggerisci(parola: String, stato: State<'_, Caricato<Correttore>>) -> Result<Vec<String>, String> {
    Ok(stato.con(|c| c.suggerisci(&parola))?.unwrap_or_default())
}

#[tauri::command]
async fn aggiungi_parola(parola: String, stato: State<'_, Caricato<Correttore>>) -> Result<(), String> {
    stato.con_mut(|c| c.aggiungi(&parola))?.unwrap_or(Ok(()))
}

/// Le voci del thesaurus per la parola; `None` finché non è caricato.
#[tauri::command]
async fn sinonimi(parola: String, stato: State<'_, Caricato<Sinonimi>>) -> Result<Option<Vec<Voce>>, String> {
    stato.con(|s| s.cerca(&parola))
}

/// La copia di sicurezza del documento con modifiche non salvate (vedi recupero.rs).
#[tauri::command]
async fn salva_copia(copia: Copia, sessione: State<'_, Option<Sessione>>) -> Result<(), String> {
    sessione.as_ref().map_or(Ok(()), |s| s.salva(&copia))
}

#[tauri::command]
fn cancella_copia(sessione: State<'_, Option<Sessione>>) {
    if let Some(s) = sessione.as_ref() {
        s.cancella();
    }
}

/// Le copie lasciate da un crash, la più recente per prima.
#[tauri::command]
fn copie_orfane(sessione: State<'_, Option<Sessione>>) -> Vec<Orfana> {
    sessione.as_ref().map(Sessione::orfane).unwrap_or_default()
}

#[tauri::command]
fn dimentica_copia(id: String, sessione: State<'_, Option<Sessione>>) {
    if let Some(s) = sessione.as_ref() {
        s.dimentica(&id);
    }
}

#[tauri::command]
fn leggi_impostazioni() -> Option<String> {
    dati::leggi_impostazioni()
}

#[tauri::command]
async fn salva_impostazioni(testo: String) -> Result<(), String> {
    dati::salva_impostazioni(&testo)
}

/// Dizionario e thesaurus, in un thread: l'editor è subito utilizzabile. Il
/// correttore arriva per primo, il thesaurus (più lento da indicizzare) dopo.
fn carica_dizionari(app: tauri::AppHandle) {
    let testi = dati::leggi_dizionario("it_IT.aff").and_then(|aff| Ok((aff, dati::leggi_dizionario("it_IT.dic")?)));
    let (aff, dic) = match testi {
        Ok(t) => t,
        Err(e) => {
            app.state::<Caricato<Correttore>>().imposta(Err(e.clone()));
            app.state::<Caricato<Sinonimi>>().imposta(Err(e));
            let _ = app.emit("correttore-pronto", ());
            return;
        }
    };
    app.state::<Caricato<Correttore>>().imposta(Correttore::carica(&aff, &dic));
    let _ = app.emit("correttore-pronto", ());
    let sinonimi = dati::leggi_dizionario("th_it_IT_v2.dat").map(|th| Sinonimi::nuovo(&aff, &dic, th));
    app.state::<Caricato<Sinonimi>>().imposta(sinonimi);
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Caricato::<Correttore>::default())
        .manage(Caricato::<Sinonimi>::default())
        .manage(dati::cartella_dati().map(|d| Sessione::nuova(d.join("recupero"))))
        .setup(|app| {
            let app = app.handle().clone();
            std::thread::spawn(move || carica_dizionari(app));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            apri_docx,
            salva_docx,
            file_iniziale,
            controlla,
            suggerisci,
            aggiungi_parola,
            sinonimi,
            salva_copia,
            cancella_copia,
            copie_orfane,
            dimentica_copia,
            leggi_impostazioni,
            salva_impostazioni
        ])
        .build(tauri::generate_context!())
        .expect("errore all'avvio di Screeve")
        .run(|app, evento| {
            // chiusura normale: la copia di sicurezza non serve più
            if let RunEvent::Exit = evento {
                if let Some(s) = app.state::<Option<Sessione>>().as_ref() {
                    s.chiudi();
                }
            }
        });
}
