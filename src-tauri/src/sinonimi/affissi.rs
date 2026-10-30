//! Dalla forma flessa al lemma con le regole di suffisso (SFX) del dizionario
//! Hunspell, applicate al contrario: "antiche" ← "antica" + regola Q (a → he).
//! Basta un livello di suffissi: è quanto serve per arrivare alle voci del thesaurus.

use std::collections::HashMap;

struct Regola {
    flag: char,
    togli: String,    // ciò che la regola toglie dalla radice
    aggiungi: String, // ciò che aggiunge al posto suo
    condizione: Vec<Classe>,
}

/// Un carattere della condizione: `.`, una lettera, `[abc]` o `[^abc]`.
enum Classe {
    Tutti,
    Uno(char),
    Tra(Vec<char>),
    Tranne(Vec<char>),
}

impl Classe {
    fn ammette(&self, c: char) -> bool {
        match self {
            Classe::Tutti => true,
            Classe::Uno(x) => *x == c,
            Classe::Tra(v) => v.contains(&c),
            Classe::Tranne(v) => !v.contains(&c),
        }
    }
}

pub struct Affissi {
    regole: Vec<Regola>,
    radici: HashMap<String, Vec<char>>, // parola del .dic → i suoi flag
}

impl Affissi {
    pub fn nuovo(aff: &str, dic: &str) -> Self {
        let regole = aff
            .lines()
            .filter_map(|riga| {
                let campi: Vec<&str> = riga.split_whitespace().collect();
                // "SFX flag togli aggiungi[/flag] condizione"; l'intestazione ha "Y"/"N" e un numero
                let ["SFX", flag, togli, aggiungi, condizione, ..] = campi[..] else { return None };
                let flag = flag.chars().next()?;
                let vuoto = |s: &str| if s == "0" { String::new() } else { s.to_string() };
                let aggiungi = aggiungi.split('/').next().unwrap_or_default();
                Some(Regola { flag, togli: vuoto(togli), aggiungi: vuoto(aggiungi), condizione: classi(condizione)? })
            })
            .collect();
        let radici = dic
            .lines()
            .skip(1)
            .filter(|r| !r.starts_with('/') && !r.trim().is_empty())
            .map(|r| {
                let (parola, flag) = r.trim().split_once('/').unwrap_or((r.trim(), ""));
                (parola.to_string(), flag.chars().collect())
            })
            .collect();
        Self { regole, radici }
    }

    /// Le radici del dizionario da cui la parola può derivare, compresa la parola stessa
    /// se è una radice.
    pub fn radici(&self, parola: &str) -> Vec<String> {
        let mut trovate = Vec::new();
        if self.radici.contains_key(parola) {
            trovate.push(parola.to_string());
        }
        for r in &self.regole {
            let Some(base) = parola.strip_suffix(r.aggiungi.as_str()) else { continue };
            let radice = format!("{base}{}", r.togli);
            if !radice.is_empty()
                && condizione(&r.condizione, &radice)
                && self.radici.get(&radice).is_some_and(|f| f.contains(&r.flag))
                && !trovate.contains(&radice)
            {
                trovate.push(radice);
            }
        }
        trovate
    }
}

fn classi(condizione: &str) -> Option<Vec<Classe>> {
    let mut v = Vec::new();
    let mut c = condizione.chars();
    while let Some(x) = c.next() {
        v.push(match x {
            '.' => Classe::Tutti,
            '[' => {
                let mut dentro: Vec<char> = c.by_ref().take_while(|&y| y != ']').collect();
                if dentro.first() == Some(&'^') {
                    dentro.remove(0);
                    Classe::Tranne(dentro)
                } else {
                    Classe::Tra(dentro)
                }
            }
            x => Classe::Uno(x),
        });
    }
    Some(v)
}

/// La condizione riguarda la fine della radice.
fn condizione(classi: &[Classe], radice: &str) -> bool {
    let fine: Vec<char> = radice.chars().rev().take(classi.len()).collect();
    fine.len() == classi.len() && classi.iter().rev().zip(fine).all(|(k, c)| k.ammette(c))
}
