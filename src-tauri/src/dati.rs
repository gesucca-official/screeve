//! Dove Screeve trova i suoi file: i dizionari accanto all'eseguibile, e i dati di
//! chi scrive (dizionario personale, più avanti le impostazioni) accanto
//! all'eseguibile se la cartella è scrivibile, come su una chiavetta USB, altrimenti
//! in %APPDATA%\Screeve. Su macOS l'app è un bundle (Screeve.app) che non si deve
//! modificare: i dizionari stanno in Contents/Resources e i dati in
//! ~/Library/Application Support/Screeve.

use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::OnceLock;

fn cartella_eseguibile() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(PathBuf::from)
}

/// La cartella `dizionari/`: accanto all'eseguibile, e in sviluppo quella del progetto.
pub fn cartella_dizionari() -> Option<PathBuf> {
    let accanto = cartella_eseguibile().map(|c| c.join("dizionari"));
    // nel bundle: Screeve.app/Contents/MacOS/screeve → Screeve.app/Contents/Resources
    let risorse = cfg!(target_os = "macos")
        .then(|| cartella_eseguibile().and_then(|c| c.parent().map(|p| p.join("Resources").join("dizionari"))))
        .flatten();
    let progetto = cfg!(debug_assertions).then(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dizionari"));
    risorse.into_iter().chain(accanto).chain(progetto).find(|c| c.join("it_IT.dic").is_file())
}

/// Un file della cartella `dizionari/`.
pub fn leggi_dizionario(nome: &str) -> Result<String, String> {
    let cartella = cartella_dizionari()
        .ok_or("Non trovo il dizionario italiano: la cartella «dizionari» deve stare accanto a Screeve")?;
    fs::read_to_string(cartella.join(nome)).map_err(|e| format!("Non riesco a leggere il file {nome} dei dizionari: {e}"))
}

/// La cartella dei dati di chi scrive, decisa una volta sola.
pub fn cartella_dati() -> Option<PathBuf> {
    static CARTELLA: OnceLock<Option<PathBuf>> = OnceLock::new();
    CARTELLA
        .get_or_init(|| {
            // portable solo fuori da macOS: scrivere dentro il bundle ne romperebbe la firma
            if !cfg!(target_os = "macos") {
                if let Some(c) = cartella_eseguibile().filter(|c| scrivibile(c)) {
                    return Some(c);
                }
            }
            let c = cartella_utente()?;
            fs::create_dir_all(&c).ok()?;
            Some(c)
        })
        .clone()
}

/// La cartella dei dati nel profilo dell'utente, quando non si può stare accanto all'eseguibile.
fn cartella_utente() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        Some(PathBuf::from(std::env::var_os("HOME")?).join("Library").join("Application Support").join("Screeve"))
    } else {
        Some(PathBuf::from(std::env::var_os("APPDATA")?).join("Screeve"))
    }
}

/// Le impostazioni (screeve.json), come le ha scritte l'interfaccia.
pub fn leggi_impostazioni() -> Option<String> {
    fs::read_to_string(cartella_dati()?.join("screeve.json")).ok()
}

pub fn salva_impostazioni(testo: &str) -> Result<(), String> {
    let cartella = cartella_dati().ok_or("Non trovo una cartella in cui salvare le impostazioni")?;
    let temporaneo = cartella.join("screeve.json.tmp");
    fs::write(&temporaneo, testo)
        .and_then(|_| fs::rename(&temporaneo, cartella.join("screeve.json")))
        .map_err(|e| format!("Non riesco a salvare le impostazioni: {e}"))
}

fn scrivibile(cartella: &PathBuf) -> bool {
    let prova = cartella.join(".screeve-prova");
    let ok = OpenOptions::new().create(true).truncate(true).write(true).open(&prova).is_ok();
    let _ = fs::remove_file(&prova);
    ok
}
