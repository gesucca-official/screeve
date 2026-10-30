//! Una risorsa che si carica in un thread all'avvio (correttore, sinonimi): finché
//! non è pronta i comandi rispondono `None`, e l'interfaccia aspetta l'evento.

use std::sync::{OnceLock, RwLock};

pub struct Caricato<T>(OnceLock<Result<RwLock<T>, String>>);

impl<T> Default for Caricato<T> {
    fn default() -> Self {
        Self(OnceLock::new())
    }
}

impl<T> Caricato<T> {
    pub fn imposta(&self, r: Result<T, String>) {
        let _ = self.0.set(r.map(RwLock::new));
    }

    /// `None` se non è ancora pronta, l'errore se non si è potuta caricare.
    pub fn con<R>(&self, f: impl FnOnce(&T) -> R) -> Result<Option<R>, String> {
        match self.0.get() {
            None => Ok(None),
            Some(Err(e)) => Err(e.clone()),
            Some(Ok(c)) => Ok(Some(f(&c.read().unwrap_or_else(|e| e.into_inner())))),
        }
    }

    pub fn con_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> Result<Option<R>, String> {
        match self.0.get() {
            None => Ok(None),
            Some(Err(e)) => Err(e.clone()),
            Some(Ok(c)) => Ok(Some(f(&mut c.write().unwrap_or_else(|e| e.into_inner())))),
        }
    }
}
