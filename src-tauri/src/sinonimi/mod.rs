//! Sinonimi dal thesaurus italiano di LibreOffice (formato MyThes): una riga
//! "parola|n" seguita da n significati "(categoria)|sinonimo|sinonimo…".
//!
//! Il thesaurus ha solo forme base (antico, brillare); dalla parola scritta
//! (antiche, brilla) si risale al lemma con le regole del dizionario (affissi.rs) e
//! l'interfaccia dice "da «antico»". Proporre i sinonimi già flessi è il passo dopo.

mod affissi;
#[cfg(test)]
mod test;

use affissi::Affissi;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Voce {
    pub lemma: String,
    pub significati: Vec<Significato>,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Significato {
    /// "aggettivo", "verbo"… con il senso, se il thesaurus lo indica: "aggettivo · avvilito"
    pub categoria: String,
    pub sinonimi: Vec<String>,
}

pub struct Sinonimi {
    testo: String,
    indice: HashMap<String, (usize, usize)>, // lemma → (byte del primo significato, quanti)
    affissi: Affissi,
}

impl Sinonimi {
    pub fn nuovo(aff: &str, dic: &str, thesaurus: String) -> Self {
        let mut indice = HashMap::new();
        let mut pos = 0;
        let mut righe = thesaurus.split_inclusive('\n');
        // la prima riga è la codifica
        if let Some(r) = righe.next() {
            pos += r.len();
        }
        let mut restanti = 0;
        for r in righe {
            let inizio = pos;
            pos += r.len();
            if restanti > 0 {
                restanti -= 1;
                continue;
            }
            let Some((parola, n)) = r.trim_end().split_once('|') else { continue };
            let Ok(n) = n.parse::<usize>() else { continue };
            indice.insert(parola.to_string(), (inizio + r.len(), n));
            restanti = n;
        }
        Self { testo: thesaurus, indice, affissi: Affissi::nuovo(aff, dic) }
    }

    /// Le voci del thesaurus per la parola scritta: prima la parola stessa, poi i
    /// lemmi da cui può derivare.
    pub fn cerca(&self, parola: &str) -> Vec<Voce> {
        self.lemmi(parola).into_iter().filter_map(|l| self.voce(&l)).collect()
    }

    fn lemmi(&self, parola: &str) -> Vec<String> {
        let p = parola.trim().replace('’', "'");
        // dell'acqua → acqua
        let p = match p.rsplit_once('\'') {
            Some((_, dopo)) if !dopo.is_empty() => dopo.to_string(),
            _ => p,
        };
        let mut candidati = Vec::new();
        for forma in [p.clone(), p.to_lowercase()] {
            let radici = self.affissi.radici(&forma);
            // case → casa prima di caso: nel dizionario "casa" non è una radice ma una
            // forma di "caso"; il singolare vale se viene dalla stessa radice
            let singolari = singolari(&forma)
                .into_iter()
                .filter(|s| self.affissi.radici(s).iter().any(|r| radici.contains(r)));
            for r in std::iter::once(forma.clone()).chain(singolari).chain(radici.clone()) {
                // il thesaurus ha gli aggettivi al maschile: antica → antico
                let maschile = r.strip_suffix('a').map(|b| format!("{b}o"));
                for c in std::iter::once(r).chain(maschile) {
                    if self.indice.contains_key(&c) && !candidati.contains(&c) {
                        candidati.push(c);
                    }
                }
            }
        }
        candidati
    }

    fn voce(&self, lemma: &str) -> Option<Voce> {
        let &(inizio, n) = self.indice.get(lemma)?;
        let significati = self.testo[inizio..]
            .lines()
            .take(n)
            .filter_map(|riga| {
                let mut campi = riga.split('|');
                let categoria = categoria(campi.next()?);
                let sinonimi: Vec<String> =
                    campi.map(str::trim).filter(|s| !s.is_empty() && *s != lemma).map(String::from).collect();
                (!sinonimi.is_empty()).then_some(Significato { categoria, sinonimi })
            })
            .collect::<Vec<_>>();
        (!significati.is_empty()).then(|| Voce { lemma: lemma.to_string(), significati })
    }
}

/// I possibili singolari di un plurale: antiche → antica, case → casa, poeti → poeta.
fn singolari(p: &str) -> Vec<String> {
    let cambi: &[(&str, &[&str])] = &[
        ("che", &["ca"]),
        ("ghe", &["ga"]),
        ("chi", &["co", "ca"]),
        ("ghi", &["go", "ga"]),
        ("e", &["a"]),
        ("i", &["o", "e", "a"]),
    ];
    cambi
        .iter()
        .find_map(|(fine, nuove)| p.strip_suffix(fine).map(|b| nuove.iter().map(|n| format!("{b}{n}")).collect()))
        .unwrap_or_default()
}

/// "(agg. Avvilito)" → "aggettivo · avvilito"
fn categoria(grezza: &str) -> String {
    let dentro = grezza.trim().trim_start_matches('(').trim_end_matches(')').trim();
    let (sigla, senso) = match dentro.find(". ") {
        Some(i) => (&dentro[..=i], dentro[i + 2..].trim()),
        None => (dentro, ""),
    };
    // gli invariabili valgono come gli altri: "s.f.inv." → "s.f."
    let nome = match sigla.replace("inv.", "").as_str() {
        "agg." => "aggettivo",
        "avv." => "avverbio",
        "v." => "verbo",
        "s.m." => "sostantivo maschile",
        "s.f." => "sostantivo femminile",
        "s.m.pl." => "sostantivo maschile plurale",
        "s.f.pl." => "sostantivo femminile plurale",
        "s.m.f." => "sostantivo",
        "v.tr." => "verbo transitivo",
        "v.intr." => "verbo intransitivo",
        "cong." => "congiunzione",
        "prep." => "preposizione",
        "pron." => "pronome",
        "inter." | "inter" => "interiezione",
        "loc." => "locuzione",
        _ => sigla,
    };
    if senso.is_empty() { nome.to_string() } else { format!("{nome} · {}", senso.to_lowercase()) }
}
