//! Lettura e scrittura dei file .docx.
//!
//! Il modello è lo stesso dello schema dell'editor: paragrafi e titoli di due livelli,
//! fatti di "run" di testo con grassetto e corsivo. L'OOXML lo scriviamo a mano, così
//! sappiamo esattamente cosa finisce nel file; in lettura teniamo solo ciò che il
//! modello sa rappresentare e contiamo il resto, per avvisare chi scrive.

mod lettura;
mod modello;
mod scrittura;
#[cfg(test)]
mod test;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

pub use lettura::leggi;
pub use modello::Modello;
pub use scrittura::scrivi;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tipo {
    Paragrafo,
    Titolo,
}

/// Un paragrafo o un titolo. `livello` vale 1 (Titolo) o 2 (Sottotitolo) per i titoli, 0 per i paragrafi.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Blocco {
    pub tipo: Tipo,
    #[serde(default)]
    pub livello: u8,
    pub run: Vec<Run>,
    /// l'interno del <w:pPr> originale; solo per i file aperti (vedi modello.rs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formato: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub testo: String,
    #[serde(default)]
    pub grassetto: bool,
    #[serde(default)]
    pub corsivo: bool,
    #[serde(default)]
    pub sottolineato: bool,
    #[serde(default)]
    pub barrato: bool,
    /// l'interno del <w:rPr> originale, senza grassetto e corsivo
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formato: Option<String>,
}

/// Un tipo di elemento che Screeve non gestisce, trovato nel file aperto. La chiave
/// (es. "tabelle") la traduce in parole l'interfaccia.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct NonSupportato {
    pub elemento: &'static str,
    pub quanti: u32,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Documento {
    pub blocchi: Vec<Blocco>,
    pub non_supportati: Vec<NonSupportato>,
}

impl Blocco {
    pub fn paragrafo(run: Vec<Run>) -> Self {
        Blocco { tipo: Tipo::Paragrafo, livello: 0, run, formato: None }
    }

    pub fn titolo(livello: u8, run: Vec<Run>) -> Self {
        Blocco { tipo: Tipo::Titolo, livello, run, formato: None }
    }

    /// 1 o 2 per i titoli, 0 per i paragrafi; un livello fuori misura diventa Sottotitolo.
    pub fn livello_titolo(&self) -> u8 {
        match self.tipo {
            Tipo::Paragrafo => 0,
            Tipo::Titolo => self.livello.clamp(1, 2),
        }
    }
}

impl Run {
    pub fn new(testo: &str, grassetto: bool, corsivo: bool) -> Self {
        Run { testo: testo.into(), grassetto, corsivo, sottolineato: false, barrato: false, formato: None }
    }

    /// Stessi grassetto, corsivo, sottolineato e barrato.
    pub fn stesso_aspetto(&self, altro: &Run) -> bool {
        (self.grassetto, self.corsivo, self.sottolineato, self.barrato)
            == (altro.grassetto, altro.corsivo, altro.sottolineato, altro.barrato)
    }
}

/// Apre un file .docx dal disco: il documento, e il modello per risalvarlo con la sua
/// impaginazione.
pub fn apri(percorso: &Path) -> Result<(Documento, Option<Modello>), String> {
    let nome = nome_file(percorso);
    let dati = fs::read(percorso).map_err(|e| format!("Non riesco a leggere «{nome}»: {}", errore_io(&e)))?;
    let documento = leggi(Cursor::new(&dati)).map_err(|e| format!("Non riesco ad aprire «{nome}»: {e}"))?;
    Ok((documento, Modello::nuovo(dati)))
}

/// Il modello di un file già su disco, per un documento recuperato dopo un crash.
pub fn modello_da(percorso: &Path) -> Option<Modello> {
    Modello::nuovo(fs::read(percorso).ok()?)
}

/// Salva in modo atomico: scrive accanto al file un temporaneo e poi lo rinomina sopra
/// l'originale, così un errore a metà non lascia mai un documento troncato.
pub fn salva(percorso: &Path, blocchi: &[Blocco], modello: Option<&Modello>) -> Result<(), String> {
    let nome = nome_file(percorso);
    let errore = |e: std::io::Error| format!("Non riesco a salvare «{nome}»: {}", errore_io(&e));

    let mut dati = Cursor::new(Vec::new());
    match modello {
        Some(m) => m.scrivi(&mut dati, blocchi),
        None => scrivi(&mut dati, blocchi),
    }
    .map_err(|e| format!("Non riesco a salvare «{nome}»: {e}"))?;
    // meglio un errore che un documento illeggibile: il file sul disco resta com'era
    dati.set_position(0);
    leggi(&mut dati).map_err(|e| format!("Non riesco a salvare «{nome}»: {e}"))?;

    let cartella = match percorso.parent() {
        Some(c) if !c.as_os_str().is_empty() => c,
        _ => Path::new("."),
    };
    let temporaneo = cartella.join(format!(".~{nome}.{}.tmp", std::process::id()));
    let scrittura = (|| {
        let mut f = fs::File::create(&temporaneo)?;
        f.write_all(dati.get_ref())?;
        f.sync_all()?;
        drop(f);
        fs::rename(&temporaneo, percorso)
    })();
    if let Err(e) = scrittura {
        let _ = fs::remove_file(&temporaneo);
        return Err(errore(e));
    }
    Ok(())
}

fn nome_file(percorso: &Path) -> String {
    percorso.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn errore_io(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    // 32 = ERROR_SHARING_VIOLATION: tipicamente il documento è aperto in Word
    if cfg!(windows) && e.raw_os_error() == Some(32) {
        return "il file è aperto in un altro programma".into();
    }
    match e.kind() {
        NotFound => "il file o la cartella non esiste".into(),
        PermissionDenied => "accesso negato (il file è aperto in un altro programma o è di sola lettura?)".into(),
        _ => e.to_string(),
    }
}
