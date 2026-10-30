use super::*;
use std::sync::LazyLock;

// il dizionario vero, caricato una volta per tutti i test
static C: LazyLock<Correttore> = LazyLock::new(|| {
    let cartella = Path::new(env!("CARGO_MANIFEST_DIR")).join("dizionari");
    let leggi = |n: &str| fs::read_to_string(cartella.join(n)).unwrap();
    Correttore::nuovo(&leggi("it_IT.aff"), &leggi("it_IT.dic")).unwrap()
});

fn refusi(testo: &str) -> Vec<String> {
    C.controlla(testo).into_iter().map(|r| r.parola).collect()
}

#[test]
fn testo_corretto() {
    let testo = "Quella sera, però, il vento accelerava da nord: «È tardi», disse Giovanni, \
                 e sull’orizzonte comparve una luce. Dell'acqua, un po' di pane, c'è anch'io.";
    assert_eq!(refusi(testo), Vec::<String>::new());
}

#[test]
fn refusi_trovati() {
    assert_eq!(refusi("il vento accellerava, perchè piu forte"), ["accellerava", "perchè", "piu"]);
}

#[test]
fn regole_italiane() {
    assert_eq!(refusi("Qual'è il punto? Un po di sale, un'altro giorno, pò di più."), ["Qual'è", "po", "un'altro", "pò"]);
    assert_eq!(C.suggerisci("Qual'è")[0], "Qual è");
    assert_eq!(C.suggerisci("po")[0], "po'");
    assert_eq!(C.suggerisci("un'altro")[0], "un altro");
    assert_eq!(C.suggerisci("poichè")[0], "poiché");
    assert_eq!(C.suggerisci("Affinchè")[0], "Affinché");
    // Po il fiume resta
    assert_eq!(refusi("lungo il Po"), Vec::<String>::new());
}

#[test]
fn apostrofo_tipografico() {
    assert_eq!(refusi("dell’acqua e l’uomo"), Vec::<String>::new());
    assert_eq!(C.suggerisci("qual’è")[0], "qual è");
    // la virgoletta singola di chiusura non si attacca alla parola
    assert_eq!(refusi("disse 'ciao' e andò"), Vec::<String>::new());
}

#[test]
fn posizioni_utf16() {
    // "È" e "«" sono un'unità UTF-16 ciascuno, come in JavaScript
    let r = C.controlla("«È accellerava»");
    assert_eq!(r, [Refuso { inizio: 3, fine: 14, parola: "accellerava".into() }]);
    // fuori dal piano base, due unità
    let r = C.controlla("😀 accellerava");
    assert_eq!((r[0].inizio, r[0].fine), (3, 14));
}

#[test]
fn numeri_e_indirizzi_saltati() {
    assert_eq!(refusi("nel 1984 vidi 3D e 2a fila, www.esempiox.it e mario.rossix@posta.it o https://qwerty.zz/abc"), Vec::<String>::new());
}

#[test]
fn trattini() {
    assert_eq!(refusi("una e-mail e un dopo-cena"), Vec::<String>::new());
    let r = C.controlla("ciao-accellerava");
    assert_eq!(r, [Refuso { inizio: 5, fine: 16, parola: "accellerava".into() }]);
}

#[test]
fn suggerimenti() {
    let s = C.suggerisci("accellerava");
    assert_eq!(s[0], "accelerava");
    assert!(s.len() <= SUGGERIMENTI);
    assert!(!C.suggerisci("sopratutto").contains(&"sopratutto".to_string()));
}

#[test]
fn dizionario_personale() {
    let cartella = std::env::temp_dir().join(format!("screeve-personale-{}", std::process::id()));
    fs::create_dir_all(&cartella).unwrap();
    let file = cartella.join("personale.dic");
    let _ = fs::remove_file(&file);

    let mut c = Correttore::nuovo(
        &fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("dizionari/it_IT.aff")).unwrap(),
        &fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("dizionari/it_IT.dic")).unwrap(),
    )
    .unwrap();
    c.file_personale = Some(file.clone());
    assert_eq!(c.controlla("Ilvaren guardò").len(), 1);
    c.aggiungi("Ilvaren").unwrap();
    c.aggiungi("Ilvaren").unwrap(); // una volta sola nel file
    assert!(c.controlla("Ilvaren guardò").is_empty());
    assert_eq!(fs::read_to_string(&file).unwrap(), "Ilvaren\n");

    // al riavvio la parola c'è ancora
    let mut nuovo = Correttore { diz: Dictionary::new("SET UTF-8\n", "0\n").unwrap(), personali: BTreeSet::new(), file_personale: None };
    nuovo.leggi_personale(&file);
    assert!(nuovo.corretta("Ilvaren"));
    fs::remove_dir_all(&cartella).unwrap();
}
