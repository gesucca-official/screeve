//! Lettura di docx prodotti da Word, Google Documenti, LibreOffice e Screeve stesso.
//!
//! Teniamo testo, paragrafi, titoli, grassetto e corsivo; il resto (tabelle, immagini,
//! note…) lo contiamo per l'avviso all'apertura. Del testo non si perde nulla di ciò
//! che si vede: il contenuto delle tabelle diventa paragrafi, i campi il loro risultato.

use super::{Blocco, Documento, NonSupportato, Run};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::{NsReader, Reader};
use std::collections::HashMap;
use std::io::{Read, Seek};
use zip::ZipArchive;

const NS_W: [&str; 2] = [
    "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "http://purl.oclc.org/ooxml/wordprocessingml/main", // Strict Open XML
];
const NS_MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const NS_M: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/math",
    "http://purl.oclc.org/ooxml/officeDocument/math",
];

/// Gli elementi non supportati, nell'ordine in cui li elenca l'avviso.
const ELEMENTI: [&str; 15] = [
    "tabelle", "immagini", "caselle", "oggetti", "formule", "note", "commenti", "revisioni",
    "elenchi", "intestazioni", "interruzioni", "campi", "collegamenti", "sottolineature", "simboli",
];

/// Legge un .docx.
pub fn leggi<R: Read + Seek>(sorgente: R) -> Result<Documento, String> {
    let mut pacchetto = Pacchetto::apri(sorgente)?;

    let principale = pacchetto
        .relazioni("")
        .into_iter()
        .find(|(tipo, _)| tipo.ends_with("/officeDocument"))
        .map(|(_, destinazione)| destinazione)
        .unwrap_or_else(|| "word/document.xml".into());
    let documento = pacchetto
        .parte(&principale)?
        .ok_or("non è un documento di testo (manca word/document.xml)")?;

    let mut stili = Stili::default();
    let mut conteggi = Conteggi::default();
    for (tipo, destinazione) in pacchetto.relazioni(&principale) {
        let tipo = tipo.rsplit('/').next().unwrap_or_default();
        if tipo == "styles" {
            if let Some(xml) = pacchetto.parte(&destinazione)? {
                stili = Stili::leggi(&xml).map_err(danneggiato)?;
            }
        } else if tipo == "header" || tipo == "footer" {
            if let Some(xml) = pacchetto.parte(&destinazione)? {
                if ha_contenuto(&xml) {
                    conteggi.aggiungi("intestazioni");
                }
            }
        }
    }

    let mut lettore = Lettore::new(&stili, conteggi);
    lettore.leggi(&documento).map_err(danneggiato)?;
    Ok(Documento {
        blocchi: lettore.blocchi,
        non_supportati: lettore.conteggi.elenco(),
    })
}

fn danneggiato(e: quick_xml::Error) -> String {
    format!("il file è danneggiato ({e})")
}

/* ---------- pacchetto zip ---------- */

struct Pacchetto<R> {
    zip: ZipArchive<R>,
    nomi: HashMap<String, usize>, // i nomi delle parti non distinguono maiuscole e minuscole
}

impl<R: Read + Seek> Pacchetto<R> {
    fn apri(sorgente: R) -> Result<Self, String> {
        let zip = ZipArchive::new(sorgente).map_err(|_| {
            "non è un file .docx valido (i vecchi file .doc e i documenti protetti da password non si possono aprire)".to_string()
        })?;
        let nomi = zip
            .file_names()
            .enumerate()
            .filter_map(|(i, n)| n.ok().map(|n| (n.trim_start_matches('/').to_lowercase(), i)))
            .collect();
        Ok(Pacchetto { zip, nomi })
    }

    fn parte(&mut self, nome: &str) -> Result<Option<String>, String> {
        let Some(&i) = self.nomi.get(&nome.to_lowercase()) else { return Ok(None) };
        let mut byte = Vec::new();
        self.zip
            .by_index(i)
            .and_then(|mut f| f.read_to_end(&mut byte).map_err(Into::into))
            .map_err(|e| format!("il file è danneggiato ({e})"))?;
        Ok(Some(testo(byte)))
    }

    /// Le relazioni di una parte ("" per il pacchetto): (tipo, destinazione risolta).
    fn relazioni(&mut self, parte: &str) -> Vec<(String, String)> {
        let (cartella, nome) = parte.rsplit_once('/').unwrap_or(("", parte));
        let percorso = if cartella.is_empty() { format!("_rels/{nome}.rels") } else { format!("{cartella}/_rels/{nome}.rels") };
        let Ok(Some(xml)) = self.parte(&percorso) else { return Vec::new() };

        let mut reader = Reader::from_str(&xml);
        let mut relazioni = Vec::new();
        while let Ok(evento) = reader.read_event() {
            match evento {
                Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "Relationship" => {
                    if attributo(&e, "TargetMode").as_deref() == Some("External") {
                        continue;
                    }
                    if let (Some(tipo), Some(destinazione)) = (attributo(&e, "Type"), attributo(&e, "Target")) {
                        relazioni.push((tipo, risolvi(cartella, &destinazione)));
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        relazioni
    }
}

/// Le parti sono quasi sempre UTF-8; qualche programma usa UTF-16.
fn testo(byte: Vec<u8>) -> String {
    match byte.as_slice() {
        [0xFF, 0xFE, resto @ ..] => {
            String::from_utf16_lossy(&resto.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect::<Vec<_>>())
        }
        [0xFE, 0xFF, resto @ ..] => {
            String::from_utf16_lossy(&resto.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect::<Vec<_>>())
        }
        [0xEF, 0xBB, 0xBF, resto @ ..] => String::from_utf8_lossy(resto).into_owned(),
        _ => String::from_utf8(byte).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
    }
}

/// Destinazione di una relazione: assoluta ("/word/x.xml") o relativa alla cartella della parte.
fn risolvi(cartella: &str, destinazione: &str) -> String {
    let completo = match destinazione.strip_prefix('/') {
        Some(assoluta) => assoluta.to_string(),
        None if cartella.is_empty() => destinazione.to_string(),
        None => format!("{cartella}/{destinazione}"),
    };
    let mut parti: Vec<&str> = Vec::new();
    for p in completo.split('/') {
        match p {
            "" | "." => {}
            ".." => { parti.pop(); }
            p => parti.push(p),
        }
    }
    parti.join("/")
}

/// Valore di un attributo, cercato per nome locale (w:val e val sono lo stesso).
fn attributo(e: &BytesStart, nome: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == nome)
        .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok().map(|v| v.into_owned()))
}

/// Proprietà sì/no di OOXML: <w:b/> è acceso, w:val="0", "false" o "off" lo spengono.
fn acceso(e: &BytesStart) -> bool {
    !matches!(attributo(e, "val").as_deref(), Some("0" | "false" | "off"))
}

/// Un'intestazione o un piè di pagina conta solo se contiene qualcosa.
fn ha_contenuto(xml: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut in_t = false;
    while let Ok(evento) = reader.read_event() {
        match evento {
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                "drawing" | "pict" | "fldSimple" | "instrText" | "fldChar" => return true,
                "t" => in_t = true,
                _ => {}
            },
            Event::End(_) => in_t = false,
            Event::Text(t) if in_t && !t.trim().is_empty() => return true,
            Event::GeneralRef(_) if in_t => return true,
            Event::Eof => break,
            _ => {}
        }
    }
    false
}

/* ---------- stili ---------- */

#[derive(Default)]
struct Stile {
    nome: String,
    base: Option<String>,
    livello_struttura: Option<u8>, // w:outlineLvl
    grassetto: Option<bool>,
    corsivo: Option<bool>,
}

#[derive(Default)]
struct Stili {
    stili: HashMap<String, Stile>,
    paragrafo_predefinito: Option<String>,
    grassetto: Option<bool>, // da docDefaults
    corsivo: Option<bool>,
}

impl Stili {
    fn leggi(xml: &str) -> Result<Self, quick_xml::Error> {
        let mut stili = Stili::default();
        let mut reader = NsReader::from_str(xml);
        reader.config_mut().expand_empty_elements = true;
        let mut pila: Vec<String> = Vec::new();
        let mut corrente: Option<(String, Stile)> = None;

        loop {
            let (ns, evento) = reader.read_resolved_event()?;
            let w = is_w(&ns);
            match evento {
                Event::Start(e) => {
                    let nome = e.local_name().as_ref().to_string();
                    pila.push(if w { nome } else { format!("?{nome}") });
                    let percorso = |fine: &[&str]| pila.ends_with(&fine.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                    match pila.last().map(String::as_str) {
                        Some("style") => {
                            let id = attributo(&e, "styleId").unwrap_or_default();
                            let tipo = attributo(&e, "type").unwrap_or_else(|| "paragraph".into());
                            if tipo == "paragraph" && attributo(&e, "default").is_some_and(|d| d == "1" || d == "true") {
                                stili.paragrafo_predefinito = Some(id.clone());
                            }
                            corrente = Some((id, Stile::default()));
                        }
                        Some("name") if percorso(&["style", "name"]) => {
                            if let Some((_, s)) = corrente.as_mut() {
                                s.nome = attributo(&e, "val").unwrap_or_default();
                            }
                        }
                        Some("basedOn") if percorso(&["style", "basedOn"]) => {
                            if let Some((_, s)) = corrente.as_mut() {
                                s.base = attributo(&e, "val");
                            }
                        }
                        Some("outlineLvl") if percorso(&["style", "pPr", "outlineLvl"]) => {
                            if let Some((_, s)) = corrente.as_mut() {
                                s.livello_struttura = attributo(&e, "val").and_then(|v| v.parse().ok());
                            }
                        }
                        Some(p @ ("b" | "i")) => {
                            let valore = Some(acceso(&e));
                            if percorso(&["style", "rPr", p]) {
                                if let Some((_, s)) = corrente.as_mut() {
                                    if p == "b" { s.grassetto = valore } else { s.corsivo = valore }
                                }
                            } else if percorso(&["docDefaults", "rPrDefault", "rPr", p]) {
                                if p == "b" { stili.grassetto = valore } else { stili.corsivo = valore }
                            }
                        }
                        _ => {}
                    }
                }
                Event::End(_) => {
                    if pila.pop().as_deref() == Some("style") {
                        if let Some((id, mut s)) = corrente.take() {
                            if s.nome.is_empty() {
                                s.nome = id.clone();
                            }
                            stili.stili.insert(id, s);
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(stili)
    }

    /// 1 = Titolo, 2 = Sottotitolo, 0 = Testo. Il livello si riconosce dal nome dello stile
    /// (gli id possono essere localizzati, "Titolo1"), poi da outlineLvl, poi dallo stile di base.
    fn livello(&self, id: Option<&str>) -> u8 {
        let mut id = id.or(self.paragrafo_predefinito.as_deref());
        for _ in 0..32 {
            let Some(i) = id else { return 0 };
            // uno stile non definito in styles.xml ha solo l'id come indizio
            let Some(s) = self.stili.get(i) else { return livello_da_nome(i).unwrap_or(0) };
            if let Some(l) = livello_da_nome(&s.nome) {
                return l;
            }
            if let Some(l) = s.livello_struttura {
                return da_struttura(l);
            }
            id = s.base.as_deref();
        }
        0
    }

    /// Grassetto e corsivo definiti da uno stile o dai suoi stili di base (il primo che li definisce).
    fn formato(&self, id: Option<&str>) -> (Option<bool>, Option<bool>) {
        let (mut b, mut i) = (None, None);
        let mut id = id;
        for _ in 0..32 {
            let Some(s) = id.and_then(|x| self.stili.get(x)) else { break };
            b = b.or(s.grassetto);
            i = i.or(s.corsivo);
            id = s.base.as_deref();
        }
        (b, i)
    }

    /// Grassetto e corsivo effettivi di un run. La formattazione diretta vince; altrimenti
    /// stile del paragrafo e stile del carattere si combinano come in Word (si annullano
    /// a vicenda). Nei titoli lo stile del paragrafo non conta: grassetto e corsivo di un
    /// titolo sono il suo aspetto, non una marcatura del testo.
    fn effettivi(&self, run: &ProprietaRun, stile_paragrafo: Option<&str>, titolo: bool) -> (bool, bool) {
        let (pb, pi) = if titolo {
            (None, None)
        } else {
            let (b, i) = self.formato(stile_paragrafo.or(self.paragrafo_predefinito.as_deref()));
            (b.or(self.grassetto), i.or(self.corsivo))
        };
        let (cb, ci) = self.formato(run.stile.as_deref());
        let combina = |p: Option<bool>, c: Option<bool>| match (p, c) {
            (Some(p), Some(c)) => p != c,
            (p, c) => c.or(p).unwrap_or(false),
        };
        (run.grassetto.unwrap_or_else(|| combina(pb, cb)), run.corsivo.unwrap_or_else(|| combina(pi, ci)))
    }
}

pub(super) fn livello_da_nome(nome: &str) -> Option<u8> {
    let nome = nome.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    match nome.as_str() {
        "title" | "titolo" | "titolo principale" => return Some(1),
        "subtitle" | "sottotitolo" => return Some(2),
        _ => {}
    }
    ["heading", "titolo"].iter().find_map(|prefisso| {
        let n: u8 = nome.strip_prefix(prefisso)?.trim().parse().ok()?;
        (n >= 1).then_some(if n == 1 { 1 } else { 2 })
    })
}

/// outlineLvl 0 è il primo livello della struttura, 9 il corpo del testo.
fn da_struttura(livello: u8) -> u8 {
    match livello {
        0 => 1,
        1..=8 => 2,
        _ => 0,
    }
}

fn is_w(ns: &ResolveResult) -> bool {
    matches!(ns, ResolveResult::Bound(Namespace(n)) if NS_W.contains(n))
}

/* ---------- conteggi ---------- */

#[derive(Default)]
struct Conteggi([u32; ELEMENTI.len()]);

impl Conteggi {
    fn aggiungi(&mut self, elemento: &str) {
        if let Some(i) = ELEMENTI.iter().position(|e| *e == elemento) {
            self.0[i] += 1;
        }
    }

    fn elenco(&self) -> Vec<NonSupportato> {
        ELEMENTI
            .iter()
            .zip(self.0)
            .filter(|(_, n)| *n > 0)
            .map(|(e, quanti)| NonSupportato { elemento: e, quanti })
            .collect()
    }
}

/* ---------- documento ---------- */

#[derive(Clone, Default, PartialEq)]
struct ProprietaRun {
    stile: Option<String>,
    grassetto: Option<bool>,
    corsivo: Option<bool>,
    sottolineato: bool,
}

enum Pezzo {
    Testo(String, ProprietaRun),
    ACapo,
}

struct Paragrafo {
    profondita: usize,
    stile: Option<String>,
    livello_struttura: Option<u8>,
    numerato: bool,
    pezzi: Vec<Pezzo>,
}

/// Parti del documento che saltiamo per intero, e cosa contare quando finiscono.
#[derive(Clone, Copy)]
enum Salto {
    Disegno { casella: bool },
    Conta(&'static str),
    Muto,
}

struct Lettore<'s> {
    stili: &'s Stili,
    conteggi: Conteggi,
    blocchi: Vec<Blocco>,
    pila: Vec<String>,
    salto: Option<(usize, Salto)>,
    paragrafo: Option<Paragrafo>,
    run: Option<(usize, ProprietaRun)>,
    in_t: bool,
    campi: Vec<bool>, // un elemento per campo aperto: true quando siamo nel risultato
    tabelle: usize,
}

impl<'s> Lettore<'s> {
    fn new(stili: &'s Stili, conteggi: Conteggi) -> Self {
        Lettore {
            stili,
            conteggi,
            blocchi: Vec::new(),
            pila: Vec::new(),
            salto: None,
            paragrafo: None,
            run: None,
            in_t: false,
            campi: Vec::new(),
            tabelle: 0,
        }
    }

    fn leggi(&mut self, xml: &str) -> Result<(), quick_xml::Error> {
        let mut reader = NsReader::from_str(xml);
        reader.config_mut().expand_empty_elements = true;
        loop {
            let (ns, evento) = reader.read_resolved_event()?;
            let spazio = match &ns {
                ResolveResult::Bound(Namespace(n)) if NS_W.contains(n) => "",
                ResolveResult::Bound(Namespace(n)) if *n == NS_MC => "mc:",
                ResolveResult::Bound(Namespace(n)) if NS_M.contains(n) => "m:",
                _ => "?",
            };
            match evento {
                Event::Start(e) => {
                    self.pila.push(format!("{spazio}{}", e.local_name().as_ref()));
                    match self.salto {
                        Some((_, Salto::Disegno { .. })) if self.pila.last().is_some_and(|n| n == "txbxContent") => {
                            self.salto.as_mut().unwrap().1 = Salto::Disegno { casella: true };
                        }
                        Some(_) => {}
                        None => self.apri(&e),
                    }
                }
                Event::End(_) => {
                    let nome = self.pila.pop().unwrap_or_default();
                    match self.salto {
                        Some((profondita, salto)) => {
                            if self.pila.len() < profondita {
                                self.salto = None;
                                match salto {
                                    Salto::Disegno { casella: true } => self.conteggi.aggiungi("caselle"),
                                    Salto::Disegno { casella: false } => self.conteggi.aggiungi("immagini"),
                                    Salto::Conta(elemento) => self.conteggi.aggiungi(elemento),
                                    Salto::Muto => {}
                                }
                            }
                        }
                        None => self.chiudi(&nome),
                    }
                }
                Event::Text(t) => self.testo(&t.xml10_content()),
                Event::CData(t) => self.testo(&t.xml10_content()),
                Event::GeneralRef(r) => {
                    if let Ok(Some(c)) = r.resolve_char_ref() {
                        self.testo(c.encode_utf8(&mut [0; 4]));
                    } else if let Some(s) = quick_xml::escape::resolve_predefined_entity(&r) {
                        self.testo(s);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(())
    }

    /// Il percorso dell'elemento appena aperto finisce con questi nomi?
    fn sotto(&self, genitori: &[&str]) -> bool {
        let n = self.pila.len();
        n > genitori.len() && self.pila[n - 1 - genitori.len()..n - 1].iter().zip(genitori).all(|(a, b)| a == b)
    }

    fn salta(&mut self, salto: Salto) {
        self.salto = Some((self.pila.len(), salto));
    }

    fn visibile(&self) -> bool {
        self.salto.is_none() && self.campi.iter().all(|&risultato| risultato)
    }

    fn apri(&mut self, e: &BytesStart) {
        let nome = self.pila.last().cloned().unwrap_or_default();
        match nome.as_str() {
            "p" if self.paragrafo.is_none() => {
                self.paragrafo = Some(Paragrafo {
                    profondita: self.pila.len(),
                    stile: None,
                    livello_struttura: None,
                    numerato: false,
                    pezzi: Vec::new(),
                });
            }
            "pStyle" if self.sotto(&["p", "pPr"]) => {
                if let Some(p) = self.paragrafo.as_mut() {
                    p.stile = attributo(e, "val");
                }
            }
            "outlineLvl" if self.sotto(&["p", "pPr"]) => {
                if let Some(p) = self.paragrafo.as_mut() {
                    p.livello_struttura = attributo(e, "val").and_then(|v| v.parse().ok());
                }
            }
            "numId" if self.sotto(&["p", "pPr", "numPr"]) => {
                if let Some(p) = self.paragrafo.as_mut() {
                    p.numerato = attributo(e, "val").is_some_and(|v| v != "0");
                }
            }
            "pageBreakBefore" if self.sotto(&["p", "pPr"]) && acceso(e) => self.conteggi.aggiungi("interruzioni"),
            "sectPr" if self.sotto(&["p", "pPr"]) => self.conteggi.aggiungi("interruzioni"),

            "r" if self.paragrafo.is_some() && self.run.is_none() => {
                self.run = Some((self.pila.len(), ProprietaRun::default()));
            }
            "rStyle" | "b" | "i" | "u" | "strike" | "dstrike" if self.sotto(&["r", "rPr"]) => {
                if let Some((_, r)) = self.run.as_mut() {
                    match nome.as_str() {
                        "rStyle" => r.stile = attributo(e, "val"),
                        "b" => r.grassetto = Some(acceso(e)),
                        "i" => r.corsivo = Some(acceso(e)),
                        "u" => r.sottolineato |= attributo(e, "val").is_none_or(|v| v != "none"),
                        _ => r.sottolineato |= acceso(e),
                    }
                }
            }
            "t" if self.sotto(&["r"]) => self.in_t = true,
            "tab" | "ptab" if self.sotto(&["r"]) => self.testo_libero("\t"),
            "noBreakHyphen" if self.sotto(&["r"]) => self.testo_libero("-"),
            "br" | "cr" if self.sotto(&["r"]) => {
                if matches!(attributo(e, "type").as_deref(), Some("page" | "column")) {
                    self.conteggi.aggiungi("interruzioni");
                } else if self.visibile() {
                    if let Some(p) = self.paragrafo.as_mut() {
                        p.pezzi.push(Pezzo::ACapo);
                    }
                }
            }
            "sym" => self.conteggi.aggiungi("simboli"),

            // campi: del campo resta il risultato visibile, non l'istruzione
            "fldChar" => match attributo(e, "fldCharType").as_deref() {
                Some("begin") => {
                    if self.campi.is_empty() {
                        self.conteggi.aggiungi("campi");
                    }
                    self.campi.push(false);
                }
                Some("separate") => {
                    if let Some(c) = self.campi.last_mut() {
                        *c = true;
                    }
                }
                Some("end") => {
                    self.campi.pop();
                }
                _ => {}
            },
            "fldSimple" if self.campi.is_empty() => self.conteggi.aggiungi("campi"),
            "hyperlink" => self.conteggi.aggiungi("collegamenti"),

            "tbl" => {
                if self.tabelle == 0 {
                    self.conteggi.aggiungi("tabelle");
                }
                self.tabelle += 1;
            }
            "footnoteReference" | "endnoteReference" => self.conteggi.aggiungi("note"),
            "commentReference" => self.conteggi.aggiungi("commenti"),
            "altChunk" => self.conteggi.aggiungi("oggetti"),

            // revisioni: si legge il testo come se fossero tutte accettate
            "ins" | "moveTo" => self.conteggi.aggiungi("revisioni"),
            "del" | "moveFrom" | "rPrChange" | "pPrChange" | "sectPrChange" | "tblPrChange" | "trPrChange"
            | "tcPrChange" | "tblGridChange" | "numberingChange" => {
                self.conteggi.aggiungi("revisioni");
                self.salta(Salto::Muto);
            }

            "drawing" | "pict" => self.salta(Salto::Disegno { casella: false }),
            "object" => self.salta(Salto::Conta("oggetti")),
            "m:oMathPara" | "m:oMath" => self.salta(Salto::Conta("formule")),
            // le alternative per i programmi vecchi duplicano ciò che c'è in mc:Choice
            "mc:Fallback" => self.salta(Salto::Muto),
            _ => {}
        }
    }

    fn chiudi(&mut self, nome: &str) {
        match nome {
            "t" => self.in_t = false,
            "r" if self.run.as_ref().is_some_and(|(d, _)| self.pila.len() < *d) => self.run = None,
            "p" if self.paragrafo.as_ref().is_some_and(|p| self.pila.len() < p.profondita) => {
                let p = self.paragrafo.take().unwrap();
                self.run = None;
                self.paragrafo_finito(p);
            }
            "tbl" => self.tabelle = self.tabelle.saturating_sub(1),
            _ => {}
        }
    }

    fn testo(&mut self, s: &str) {
        if self.in_t {
            // un a capo dentro <w:t> Word lo mostra come uno spazio
            let s = s.replace(['\r', '\n'], " ");
            self.testo_libero(&s);
        }
    }

    fn testo_libero(&mut self, s: &str) {
        if !self.visibile() || s.is_empty() {
            return;
        }
        if let (Some(p), Some((_, r))) = (self.paragrafo.as_mut(), self.run.as_ref()) {
            match p.pezzi.last_mut() {
                Some(Pezzo::Testo(t, prop)) if prop == r => t.push_str(s),
                _ => p.pezzi.push(Pezzo::Testo(s.to_string(), r.clone())),
            }
        }
    }

    fn paragrafo_finito(&mut self, p: Paragrafo) {
        let livello = match p.livello_struttura {
            Some(l) => da_struttura(l),
            None => self.stili.livello(p.stile.as_deref()),
        };
        if p.numerato {
            self.conteggi.aggiungi("elenchi");
        }

        // Un a capo manuale divide il paragrafo in due; in un titolo diventa uno spazio,
        // altrimenti un capitolo diventerebbe due.
        let mut righe: Vec<Vec<Run>> = vec![Vec::new()];
        let mut sottolineato = false;
        for pezzo in p.pezzi {
            match pezzo {
                Pezzo::ACapo if livello == 0 => righe.push(Vec::new()),
                Pezzo::ACapo => aggiungi_run(righe.last_mut().unwrap(), " ", false, false),
                Pezzo::Testo(testo, prop) => {
                    if prop.sottolineato && !sottolineato {
                        self.conteggi.aggiungi("sottolineature");
                    }
                    sottolineato = prop.sottolineato;
                    let (b, i) = self.stili.effettivi(&prop, p.stile.as_deref(), livello > 0);
                    aggiungi_run(righe.last_mut().unwrap(), &testo, b, i);
                }
            }
        }

        for mut run in righe {
            // le tabulazioni a inizio riga erano un rientro fatto a mano
            while let Some(primo) = run.first_mut() {
                let resto = primo.testo.trim_start_matches('\t');
                if resto.is_empty() {
                    run.remove(0);
                } else {
                    primo.testo = resto.to_string();
                    break;
                }
            }
            self.blocchi.push(if livello == 0 { Blocco::paragrafo(run) } else { Blocco::titolo(livello, run) });
        }
    }
}

/// Aggiunge testo in coda, unendolo all'ultimo run se ha la stessa formattazione.
fn aggiungi_run(run: &mut Vec<Run>, testo: &str, grassetto: bool, corsivo: bool) {
    match run.last_mut() {
        Some(r) if r.grassetto == grassetto && r.corsivo == corsivo => r.testo.push_str(testo),
        _ => run.push(Run::new(testo, grassetto, corsivo)),
    }
}
