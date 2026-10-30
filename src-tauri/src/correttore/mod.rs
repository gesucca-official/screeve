//! Correttore ortografico: spellbook (compatibile Hunspell) col dizionario it_IT di
//! LibreOffice, più le regole italiane di `parole.rs` e il dizionario personale.
//!
//! Il dizionario si carica in un thread all'avvio; fino ad allora `controlla`
//! risponde "non ancora pronto" e l'interfaccia aspetta l'evento `correttore-pronto`.
//! I refusi tornano come intervalli: l'editor li disegna come decorazioni, quindi
//! non entrano mai nel documento.

mod parole;
#[cfg(test)]
mod test;

use serde::Serialize;
use spellbook::Dictionary;
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use parole::{apostrofo_noto, normalizza, parole, regola, Parola};

const SUGGERIMENTI: usize = 5;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Refuso {
    /// posizioni nel testo del paragrafo, in unità UTF-16
    pub inizio: u32,
    pub fine: u32,
    pub parola: String,
}

pub struct Correttore {
    diz: Dictionary,
    personali: BTreeSet<String>,
    file_personale: Option<PathBuf>,
}

impl Correttore {
    pub fn nuovo(aff: &str, dic: &str) -> Result<Self, String> {
        let diz = Dictionary::new(aff, dic).map_err(|e| format!("Il dizionario italiano è illeggibile: {e}"))?;
        Ok(Self { diz, personali: BTreeSet::new(), file_personale: None })
    }

    /// Carica dizionario e dizionario personale dalle loro cartelle.
    pub fn carica() -> Result<Self, String> {
        let cartella = crate::dati::cartella_dizionari()
            .ok_or("Non trovo il dizionario italiano: la cartella «dizionari» deve stare accanto a Screeve")?;
        let leggi = |nome: &str| {
            fs::read_to_string(cartella.join(nome)).map_err(|e| format!("Non riesco a leggere il dizionario {nome}: {e}"))
        };
        let mut c = Self::nuovo(&leggi("it_IT.aff")?, &leggi("it_IT.dic")?)?;
        if let Some(file) = crate::dati::cartella_dati().map(|d| d.join("personale.dic")) {
            c.leggi_personale(&file);
            c.file_personale = Some(file);
        }
        Ok(c)
    }

    /// Il dizionario personale è una parola per riga, in UTF-8.
    fn leggi_personale(&mut self, file: &Path) {
        let Ok(testo) = fs::read_to_string(file) else { return };
        for parola in testo.lines().map(str::trim).filter(|p| !p.is_empty()) {
            self.impara(parola);
        }
    }

    fn impara(&mut self, parola: &str) -> bool {
        let parola = normalizza(parola);
        // "/" in un dizionario Hunspell introduce i flag
        if parola.contains('/') || self.personali.contains(&parola) || self.diz.add(&parola).is_err() {
            return false;
        }
        self.personali.insert(parola);
        true
    }

    pub fn corretta(&self, parola: &str) -> bool {
        let p = normalizza(parola);
        regola(&p).is_none() && (self.diz.check(&p) || self.personali.contains(&p))
    }

    /// I refusi di un paragrafo.
    pub fn controlla(&self, testo: &str) -> Vec<Refuso> {
        let con_apostrofo = |p: &str| apostrofo_noto(p) || self.diz.check(p) || self.personali.contains(p);
        let mut refusi = Vec::new();
        for parola in parole(testo, con_apostrofo) {
            if self.corretta(&parola.testo) {
                continue;
            }
            // e-mail, dopo-cena: se l'insieme non è nel dizionario, conta ogni pezzo
            if parola.testo.contains('-') && regola(&parola.testo).is_none() {
                refusi.extend(pezzi(&parola).filter(|p| !self.corretta(&p.testo)).map(refuso));
            } else {
                refusi.push(refuso(parola));
            }
        }
        refusi
    }

    /// Le correzioni proposte: prima quella della regola italiana, poi il dizionario.
    pub fn suggerisci(&self, parola: &str) -> Vec<String> {
        let p = normalizza(parola);
        let mut dal_diz = Vec::new();
        self.diz.suggest(&p, &mut dal_diz);
        let tipografico = parola.contains('’');
        let mut tutti: Vec<String> = Vec::new();
        for s in regola(parola).into_iter().chain(dal_diz.into_iter().map(|s| if tipografico { s.replace('\'', "’") } else { s })) {
            if s != parola && !tutti.contains(&s) {
                tutti.push(s);
            }
        }
        tutti.truncate(SUGGERIMENTI);
        tutti
    }

    /// Aggiunge una parola al dizionario personale, anche su disco.
    pub fn aggiungi(&mut self, parola: &str) -> Result<(), String> {
        let parola = normalizza(parola.trim());
        if parola.is_empty() || !self.impara(&parola) {
            return Ok(());
        }
        let Some(file) = &self.file_personale else { return Ok(()) };
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .and_then(|mut f| writeln!(f, "{parola}"))
            .map_err(|e| format!("Non riesco a salvare il dizionario personale: {e}"))
    }
}

fn refuso(p: Parola) -> Refuso {
    Refuso { inizio: p.inizio, fine: p.fine, parola: p.testo }
}

/// I pezzi di una parola col trattino, con le loro posizioni.
fn pezzi(parola: &Parola) -> impl Iterator<Item = Parola> + '_ {
    let mut inizio = parola.inizio;
    parola.testo.split('-').map(move |pezzo| {
        let fine = inizio + pezzo.encode_utf16().count() as u32;
        let p = Parola { testo: pezzo.to_string(), inizio, fine };
        inizio = fine + 1;
        p
    })
}

/// Il correttore condiviso dall'app: vuoto finché il dizionario non è caricato,
/// poi il correttore o il motivo per cui non c'è.
#[derive(Default)]
pub struct Stato(OnceLock<Result<RwLock<Correttore>, String>>);

impl Stato {
    pub fn imposta(&self, c: Result<Correttore, String>) {
        let _ = self.0.set(c.map(RwLock::new));
    }

    /// `None` se il dizionario non è ancora pronto.
    pub fn con<R>(&self, f: impl FnOnce(&Correttore) -> R) -> Result<Option<R>, String> {
        match self.0.get() {
            None => Ok(None),
            Some(Err(e)) => Err(e.clone()),
            Some(Ok(c)) => Ok(Some(f(&c.read().unwrap_or_else(|e| e.into_inner())))),
        }
    }

    pub fn con_mut<R>(&self, f: impl FnOnce(&mut Correttore) -> R) -> Result<Option<R>, String> {
        match self.0.get() {
            None => Ok(None),
            Some(Err(e)) => Err(e.clone()),
            Some(Ok(c)) => Ok(Some(f(&mut c.write().unwrap_or_else(|e| e.into_inner())))),
        }
    }
}
