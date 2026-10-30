//! Dal testo di un paragrafo alle parole da controllare, e le regole italiane che
//! valgono più del dizionario.

/// Una parola del testo. Le posizioni sono in unità UTF-16, come le stringhe di
/// JavaScript, così l'editor le usa senza conversioni.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parola {
    pub testo: String,
    pub inizio: u32,
    pub fine: u32,
}

const APOSTROFI: [char; 2] = ['\'', '’'];

/// L'apostrofo tipografico vale come quello dritto: il dizionario conosce solo il secondo.
pub fn normalizza(parola: &str) -> String {
    parola.replace('’', "'")
}

/// Le parole di un paragrafo: lettere unite da apostrofi o trattini interni
/// (dell'acqua, e-mail). L'apostrofo finale resta attaccato solo se `con_apostrofo`
/// dice che la parola lo vuole (po', ne'), altrimenti è una virgoletta di chiusura.
/// Numeri, parole attaccate a cifre, indirizzi web ed e-mail si saltano.
pub fn parole(testo: &str, con_apostrofo: impl Fn(&str) -> bool) -> Vec<Parola> {
    let car: Vec<(usize, char)> = testo.char_indices().collect();
    // posizione UTF-16 di ogni carattere, più quella finale
    let mut u16 = Vec::with_capacity(car.len() + 1);
    let mut n = 0u32;
    for (_, c) in &car {
        u16.push(n);
        n += c.len_utf16() as u32;
    }
    u16.push(n);
    let byte = |i: usize| car.get(i).map_or(testo.len(), |&(b, _)| b);
    let lettera = |i: usize| car.get(i).is_some_and(|&(_, c)| c.is_alphabetic());
    let saltati = da_saltare(testo);

    let mut risultato = Vec::new();
    let mut i = 0;
    while i < car.len() {
        if !lettera(i) {
            i += 1;
            continue;
        }
        let inizio = i;
        let unione = |i: usize| car.get(i).is_some_and(|&(_, c)| APOSTROFI.contains(&c) || c == '-');
        while lettera(i) || (i > inizio && unione(i) && lettera(i + 1)) {
            i += 1;
        }
        let mut fine = i;
        if car.get(i).is_some_and(|&(_, c)| APOSTROFI.contains(&c)) && !lettera(i + 1) {
            let con = normalizza(&testo[byte(inizio)..byte(i + 1)]);
            if con_apostrofo(&con) {
                fine = i + 1;
                i += 1;
            }
        }
        let cifra = |j: Option<usize>| j.and_then(|j| car.get(j)).is_some_and(|&(_, c)| c.is_ascii_digit());
        let (b0, b1) = (byte(inizio), byte(fine));
        if cifra(inizio.checked_sub(1)) || cifra(Some(fine)) || saltati.iter().any(|&(a, b)| b0 >= a && b1 <= b) {
            continue;
        }
        risultato.push(Parola { testo: testo[b0..b1].to_string(), inizio: u16[inizio], fine: u16[fine] });
    }
    risultato
}

/// Intervalli (in byte) di indirizzi web ed e-mail.
fn da_saltare(testo: &str) -> Vec<(usize, usize)> {
    let mut intervalli = Vec::new();
    let mut inizio = None;
    for (i, c) in testo.char_indices().chain([(testo.len(), ' ')]) {
        if !c.is_whitespace() {
            inizio.get_or_insert(i);
            continue;
        }
        if let Some(a) = inizio.take() {
            let pezzo = &testo[a..i];
            if pezzo.contains("://") || pezzo.starts_with("www.") || pezzo.contains('@') {
                intervalli.push((a, i));
            }
        }
    }
    intervalli
}

/// Errori comuni che il dizionario non vede, o per cui propone prima altro.
/// Restituisce la forma giusta, con la maiuscola iniziale se c'era.
pub fn regola(parola: &str) -> Option<String> {
    let p = normalizza(parola);
    let minuscola = p.to_lowercase();
    let giusta = match minuscola.as_str() {
        "qual'è" => "qual è".to_string(),
        "qual'era" => "qual era".to_string(),
        "un'altro" => "un altro".to_string(),
        "pò" => "po'".to_string(),
        // "Po" è il fiume: solo la minuscola è sicuramente un errore
        "po" if p == "po" => "po'".to_string(),
        "nè" => "né".to_string(),
        "sè" => "sé".to_string(),
        // perchè, poichè, affinchè, benchè, finché, giacchè, nonchè…
        m if m.ends_with("chè") => format!("{}ché", &m[..m.len() - "chè".len()]),
        // ventitrè, trentatrè…
        m if m.len() > "trè".len() && m.ends_with("trè") => format!("{}tré", &m[..m.len() - "trè".len()]),
        _ => return None,
    };
    // apostrofo tipografico se lo usava chi scrive
    let giusta = if parola.contains('’') { giusta.replace('\'', "’") } else { giusta };
    Some(if p.chars().next().is_some_and(char::is_uppercase) { maiuscola(&giusta) } else { giusta })
}

/// Parole che vogliono l'apostrofo finale e che il dizionario potrebbe non conoscere.
pub fn apostrofo_noto(parola: &str) -> bool {
    matches!(normalizza(parola).to_lowercase().as_str(), "po'" | "ne'" | "de'" | "be'" | "mo'" | "to'")
}

pub fn maiuscola(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |p| p.to_uppercase().chain(c).collect())
}
