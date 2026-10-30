//! Dove Screeve trova i suoi file: i dizionari accanto all'eseguibile, e i dati di
//! chi scrive (dizionario personale, più avanti le impostazioni) accanto
//! all'eseguibile se la cartella è scrivibile, come su una chiavetta USB, altrimenti
//! in %APPDATA%\Screeve.

use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::OnceLock;

fn cartella_eseguibile() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(PathBuf::from)
}

/// La cartella `dizionari/`: accanto all'eseguibile, e in sviluppo quella del progetto.
pub fn cartella_dizionari() -> Option<PathBuf> {
    let accanto = cartella_eseguibile().map(|c| c.join("dizionari"));
    let progetto = cfg!(debug_assertions).then(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dizionari"));
    accanto.into_iter().chain(progetto).find(|c| c.join("it_IT.dic").is_file())
}

/// La cartella dei dati di chi scrive, decisa una volta sola.
pub fn cartella_dati() -> Option<PathBuf> {
    static CARTELLA: OnceLock<Option<PathBuf>> = OnceLock::new();
    CARTELLA
        .get_or_init(|| {
            if let Some(c) = cartella_eseguibile().filter(|c| scrivibile(c)) {
                return Some(c);
            }
            let c = PathBuf::from(std::env::var_os("APPDATA")?).join("Screeve");
            fs::create_dir_all(&c).ok()?;
            Some(c)
        })
        .clone()
}

fn scrivibile(cartella: &PathBuf) -> bool {
    let prova = cartella.join(".screeve-prova");
    let ok = OpenOptions::new().create(true).truncate(true).write(true).open(&prova).is_ok();
    let _ = fs::remove_file(&prova);
    ok
}
