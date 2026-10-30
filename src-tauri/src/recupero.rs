//! Recupero dopo un crash. Mentre chi scrive ha modifiche non salvate, l'interfaccia
//! manda qui una copia del documento, che finisce in `recupero/<sessione>.json` nella
//! cartella dei dati. Il .docx non si tocca mai: salvare resta una scelta di chi scrive.
//!
//! Ogni sessione tiene un lucchetto su `<sessione>.lock` finché l'app è aperta; se
//! si chiude bene cancella la sua copia. All'avvio una copia il cui lucchetto è
//! libero viene da una sessione finita male: la si propone. Con due finestre aperte
//! insieme, quella dell'altra è bloccata e resta sua.

use crate::docx::Blocco;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Copia {
    /// il file di partenza; `None` per un documento mai salvato
    pub percorso: Option<String>,
    pub nome: String,
    /// copia di un documento con elementi persi, non ancora salvata (vedi file.ts)
    #[serde(default)]
    pub da_salvare: bool,
    pub blocchi: Vec<Blocco>,
    /// millisecondi dal 1970
    pub quando: u64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Orfana {
    pub id: String,
    pub copia: Copia,
}

pub struct Sessione {
    cartella: PathBuf,
    id: String,
    _lucchetto: Option<File>,
}

impl Sessione {
    /// Una sessione nuova in `cartella` (di solito `<dati>/recupero`).
    pub fn nuova(cartella: PathBuf) -> Self {
        let ora = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let id = format!("{ora}-{}", std::process::id());
        let lucchetto = fs::create_dir_all(&cartella)
            .and_then(|_| File::create(cartella.join(format!("{id}.lock"))))
            .ok()
            .filter(|f| f.try_lock().is_ok());
        Self { cartella, id, _lucchetto: lucchetto }
    }

    fn file(&self, id: &str) -> PathBuf {
        self.cartella.join(format!("{id}.json"))
    }

    /// Scrive la copia: prima un temporaneo, poi lo rinomina, come per i .docx.
    pub fn salva(&self, copia: &Copia) -> Result<(), String> {
        let testo = serde_json::to_vec(copia).map_err(|e| e.to_string())?;
        let finale = self.file(&self.id);
        let temporaneo = self.cartella.join(format!("{}.tmp", self.id));
        let scrittura = (|| {
            let mut f = File::create(&temporaneo)?;
            f.write_all(&testo)?;
            f.sync_all()?;
            drop(f);
            fs::rename(&temporaneo, &finale)
        })();
        scrittura.map_err(|e| format!("Non riesco a scrivere la copia di sicurezza: {e}"))
    }

    pub fn cancella(&self) {
        let _ = fs::remove_file(self.file(&self.id));
    }

    /// Le copie lasciate da sessioni chiuse male, la più recente per prima.
    pub fn orfane(&self) -> Vec<Orfana> {
        let Ok(voci) = fs::read_dir(&self.cartella) else { return Vec::new() };
        let mut orfane: Vec<Orfana> = voci
            .filter_map(|v| {
                let percorso = v.ok()?.path();
                let id = percorso.file_stem()?.to_str()?.to_string();
                if percorso.extension()? != "json" || id == self.id || in_uso(&self.cartella, &id) {
                    return None;
                }
                let copia = serde_json::from_slice(&fs::read(&percorso).ok()?).ok()?;
                Some(Orfana { id, copia })
            })
            .collect();
        orfane.sort_by(|a, b| b.copia.quando.cmp(&a.copia.quando));
        orfane
    }

    /// Dopo che chi scrive ha recuperato o scartato una copia orfana.
    pub fn dimentica(&self, id: &str) {
        if id == self.id || id.contains(['/', '\\', '.']) || in_uso(&self.cartella, id) {
            return;
        }
        let _ = fs::remove_file(self.file(id));
        let _ = fs::remove_file(self.cartella.join(format!("{id}.lock")));
    }
}

impl Sessione {
    /// Chiusura normale dell'app: la copia non serve più.
    pub fn chiudi(&self) {
        self.cancella();
        let _ = fs::remove_file(self.cartella.join(format!("{}.lock", self.id)));
    }
}

impl Drop for Sessione {
    fn drop(&mut self) {
        self.chiudi();
    }
}

/// Il lucchetto di quella sessione è preso: l'app è ancora aperta.
fn in_uso(cartella: &Path, id: &str) -> bool {
    OpenOptions::new()
        .write(true)
        .open(cartella.join(format!("{id}.lock")))
        .is_ok_and(|f| f.try_lock().is_err())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::docx::Run;

    fn cartella(nome: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("screeve-recupero-{nome}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        c
    }

    fn copia(testo: &str, quando: u64) -> Copia {
        Copia {
            percorso: None,
            nome: "Senza titolo".into(),
            da_salvare: false,
            blocchi: vec![Blocco::paragrafo(vec![Run::new(testo, false, true)])],
            quando,
        }
    }

    #[test]
    fn crash_e_recupero() {
        let c = cartella("crash");
        let mut prima = Sessione::nuova(c.clone());
        prima.salva(&copia("Era una notte buia", 1)).unwrap();
        // un crash: il sistema libera il lucchetto, ma nessuno cancella la copia
        drop(prima._lucchetto.take());
        std::mem::forget(prima);

        let dopo = Sessione::nuova(c.clone());
        let orfane = dopo.orfane();
        assert_eq!(orfane.len(), 1);
        assert_eq!(orfane[0].copia, copia("Era una notte buia", 1));

        dopo.dimentica(&orfane[0].id);
        assert!(dopo.orfane().is_empty());
        drop(dopo);
        assert_eq!(fs::read_dir(&c).unwrap().count(), 0);
        fs::remove_dir_all(&c).unwrap();
    }

    #[test]
    fn chiusura_pulita() {
        let c = cartella("pulita");
        let s = Sessione::nuova(c.clone());
        s.salva(&copia("bozza", 1)).unwrap();
        drop(s);
        assert!(Sessione::nuova(c.clone()).orfane().is_empty());
        assert_eq!(fs::read_dir(&c).unwrap().count(), 0);
        fs::remove_dir_all(&c).unwrap();
    }

    #[test]
    fn sessione_aperta_non_e_orfana() {
        let c = cartella("aperta");
        let altra = Sessione::nuova(c.clone());
        altra.salva(&copia("dell'altra finestra", 1)).unwrap();
        let mia = Sessione::nuova(c.clone());
        assert!(mia.orfane().is_empty());
        mia.dimentica(&altra.id); // non deve cancellare la copia di una sessione viva
        assert!(altra.file(&altra.id).is_file());
        drop((altra, mia));
        fs::remove_dir_all(&c).unwrap();
    }

    #[test]
    fn la_piu_recente_per_prima() {
        let c = cartella("ordine");
        fs::create_dir_all(&c).unwrap();
        for (id, quando) in [("1-1", 10), ("2-2", 30), ("3-3", 20)] {
            fs::write(c.join(format!("{id}.json")), serde_json::to_vec(&copia(id, quando)).unwrap()).unwrap();
        }
        let s = Sessione::nuova(c.clone());
        let ordine: Vec<u64> = s.orfane().iter().map(|o| o.copia.quando).collect();
        assert_eq!(ordine, [30, 20, 10]);
        drop(s);
        fs::remove_dir_all(&c).unwrap();
    }
}
