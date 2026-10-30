use super::*;
use std::sync::LazyLock;

// il thesaurus vero, caricato una volta per tutti i test
static S: LazyLock<Sinonimi> = LazyLock::new(|| {
    let cartella = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dizionari");
    let leggi = |n: &str| std::fs::read_to_string(cartella.join(n)).unwrap();
    Sinonimi::nuovo(&leggi("it_IT.aff"), &leggi("it_IT.dic"), leggi("th_it_IT_v2.dat"))
});

fn lemmi(parola: &str) -> Vec<String> {
    S.cerca(parola).into_iter().map(|v| v.lemma).collect()
}

#[test]
fn forma_base() {
    let v = S.cerca("antico");
    assert_eq!(v[0].lemma, "antico");
    assert_eq!(v[0].significati[0].categoria, "aggettivo");
    assert!(v[0].significati[0].sinonimi.contains(&"vecchio".to_string()));
}

#[test]
fn dalla_forma_flessa_al_lemma() {
    assert!(lemmi("antiche").contains(&"antico".to_string()));
    assert!(lemmi("brilla").contains(&"brillare".to_string()));
    assert!(lemmi("brillavano").contains(&"brillare".to_string()));
    assert!(lemmi("vecchie").contains(&"vecchio".to_string()));
    assert_eq!(lemmi("case")[0], "casa");
    assert!(lemmi("risplende").contains(&"risplendere".to_string()));
}

#[test]
fn maiuscola_ed_elisione() {
    assert_eq!(lemmi("Antiche"), lemmi("antiche"));
    assert!(lemmi("dell'acqua").contains(&"acqua".to_string()));
    assert!(lemmi("l’uomo").contains(&"uomo".to_string()));
}

#[test]
fn senso_nella_categoria() {
    let v = S.cerca("abbacchiato");
    assert_eq!(v[0].significati[0].categoria, "aggettivo · avvilito");
}

#[test]
fn il_lemma_non_e_sinonimo_di_se_stesso() {
    for v in S.cerca("brillare") {
        for s in v.significati {
            assert!(!s.sinonimi.contains(&v.lemma));
        }
    }
}

#[test]
fn parola_sconosciuta() {
    assert!(S.cerca("xqzw").is_empty());
    assert!(S.cerca("").is_empty());
}

