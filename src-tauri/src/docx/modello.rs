//! Il file aperto fa da modello per il salvataggio: il pacchetto originale resta com'è
//! (stili, margini, tema, intestazioni e piè di pagina) e si rigenera solo il corpo di
//! document.xml. Paragrafi e run portano con sé le loro proprietà originali (`formato`),
//! così sopravvive anche la formattazione diretta, che Google Documenti usa ovunque.
//! `Some("")` è testo del file senza proprietà proprie e resta così; `None` è testo
//! nuovo, e prende l'aspetto di ciò che ha intorno.
//! Un blocco che ha cambiato tipo nell'editor (Testo → Titolo e ritorno) tiene il suo
//! `formato` così com'era: lo stile adatto si sistema qui, a ogni salvataggio.

use super::lettura::{attributo, Pacchetto, ProprietaRun, Stili};
use super::scrittura::{run, INTESTAZIONE, STILE_TITOLO_1, STILE_TITOLO_2};
use super::{Blocco, Run};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::borrow::Cow;
use std::io::{Cursor, Seek, Write};
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::{CompressionMethod, ZipArchive};

/// Ciò che dell'originale non torna nel file perché Screeve non lo conserva nel testo:
/// lo stesso che l'avviso all'apertura dichiara perso.
const VIA_DA_PPR: [&str; 4] = ["sectPr", "numPr", "pageBreakBefore", "pPrChange"];
// grassetto e corsivo stanno nei mark; il resto è rumore. Sottolineato e barrato restano,
// per non perdere una sottolineatura doppia o colorata: li toglie la scrittura se serve.
const VIA_DA_RPR: [&str; 11] =
    ["b", "bCs", "i", "iCs", "rPrChange", "ins", "del", "moveFrom", "moveTo", "noProof", "webHidden"];

pub struct Modello {
    byte: Vec<u8>,
    principale: String,
    parte_stili: Option<String>,
    stili: Stili,
    /// document.xml fino a <w:body> compreso
    apertura: String,
    /// document.xml dall'ultimo <w:sectPr> (margini, intestazioni) alla fine
    coda: String,
}

impl Modello {
    /// `None` se il file non si lascia usare come modello: si salverà come un file nuovo.
    pub fn nuovo(byte: Vec<u8>) -> Option<Self> {
        let mut pacchetto = Pacchetto::apri(Cursor::new(&byte)).ok()?;
        let principale = pacchetto.principale();
        let xml = pacchetto.parte(&principale).ok()??;
        let (fine_apertura, inizio_coda) = corpo(&xml)?;
        let parte_stili = pacchetto
            .relazioni(&principale)
            .into_iter()
            .find(|(tipo, _)| tipo.ends_with("/styles"))
            .map(|(_, destinazione)| destinazione);
        let stili = parte_stili
            .as_ref()
            .and_then(|p| pacchetto.parte(p).ok().flatten())
            .and_then(|s| Stili::leggi(&s).ok())
            .unwrap_or_default();
        drop(pacchetto);
        let apertura = apertura(&xml[..fine_apertura])?;
        let coda = xml[inizio_coda..].to_string();
        Some(Self { byte, principale, parte_stili, stili, apertura, coda })
    }

    pub fn scrivi<W: Write + Seek>(&self, destinazione: W, blocchi: &[Blocco]) -> zip::result::ZipResult<()> {
        let mut originale = ZipArchive::new(Cursor::new(&self.byte))?;
        let mut zip = ZipWriter::new(destinazione);
        let opzioni = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let (documento, stili_mancanti) = self.documento(blocchi);
        for i in 0..originale.len() {
            let file = originale.by_index_raw(i)?;
            let nome = file.name()?.to_string();
            let parte = nome.trim_start_matches('/').to_lowercase();
            if parte == self.principale.to_lowercase() {
                drop(file);
                zip.start_file(nome, opzioni)?;
                zip.write_all(documento.as_bytes())?;
            } else if stili_mancanti && self.parte_stili.as_ref().is_some_and(|p| p.to_lowercase() == parte) {
                drop(file);
                let aggiornati = self.con_stili_titolo();
                zip.start_file(nome, opzioni)?;
                zip.write_all(aggiornati.as_bytes())?;
            } else {
                // le altre parti restano identiche, byte per byte
                zip.raw_copy_file(file)?;
            }
        }
        zip.finish()?;
        Ok(())
    }

    /// Il nuovo document.xml, e se un titolo nuovo ha bisogno di stili che il file non ha.
    fn documento(&self, blocchi: &[Blocco]) -> (String, bool) {
        let mut s = String::with_capacity(self.apertura.len() + self.coda.len() + blocchi.len() * 300);
        s.push_str(&self.apertura);
        let mut stili_mancanti = false;
        // un paragrafo nuovo prende l'aspetto di quello che lo precede; un titolo tornato
        // a Testo no: il suo aspetto non è quello del testo
        let di_testo = |b: &&Blocco| b.livello_titolo() == 0 && b.formato.as_deref().is_some_and(|f| self.livello_di(f) == 0);
        let mut riferimento = blocchi.iter().find(di_testo);
        for b in blocchi {
            let livello = b.livello_titolo();
            let titolo = livello > 0;
            if di_testo(&b) {
                riferimento = Some(b);
            }
            let formato_riferimento = riferimento.and_then(|r| r.formato.as_deref());
            let ppr: Option<Cow<str>> = match b.formato.as_deref() {
                // il tipo è quello che il formato dice: resta com'è
                Some(f) if self.livello_di(f) == livello => Some(Cow::Borrowed(f)),
                f if titolo => {
                    let id = self.stili.id_titolo(livello);
                    stili_mancanti |= id.is_none();
                    let id = id.unwrap_or(format!("Heading{livello}"));
                    Some(Cow::Owned(con_stile(f.unwrap_or(""), Some(&id))))
                }
                // un titolo tornato a Testo: le sue proprietà con lo stile del testo vicino
                Some(f) if !con_stile(f, None).is_empty() => {
                    let stile = formato_riferimento.and_then(|r| valore_figlio(r, "pStyle"));
                    Some(Cow::Owned(con_stile(f, stile.as_deref())))
                }
                _ => formato_riferimento.map(Cow::Borrowed),
            };
            let ppr = ppr.as_deref();
            let stile_paragrafo = ppr.and_then(|p| valore_figlio(p, "pStyle"));
            let formato_run = |b: &Blocco| b.run.iter().find_map(|r| r.formato.clone());
            let predefinito = formato_run(b).or_else(|| if titolo { None } else { riferimento.and_then(formato_run) });

            s.push_str("<w:p>");
            if let Some(p) = ppr.filter(|p| !p.is_empty()) {
                s.push_str("<w:pPr>");
                s.push_str(p);
                s.push_str("</w:pPr>");
            }
            for r in &b.run {
                let formato = r.formato.as_deref().or(predefinito.as_deref()).unwrap_or("");
                let proprieta = ProprietaRun::di_stile(valore_figlio(formato, "rStyle"));
                let base = self.stili.effettivi(&proprieta, stile_paragrafo.as_deref(), titolo);
                let decorazioni = self.stili.decorazioni(&proprieta, stile_paragrafo.as_deref(), titolo);
                run(&mut s, r, &proprieta_run(formato, r, base, decorazioni));
            }
            s.push_str("</w:p>");
        }
        s.push_str(&self.coda);
        (s, stili_mancanti)
    }

    /// Il livello di titolo che queste proprietà danno a un paragrafo, come in lettura.
    fn livello_di(&self, ppr: &str) -> u8 {
        let struttura = valore_figlio(ppr, "outlineLvl").and_then(|v| v.parse().ok());
        self.stili.livello_paragrafo(valore_figlio(ppr, "pStyle").as_deref(), struttura)
    }

    fn con_stili_titolo(&self) -> String {
        let mut pacchetto = Pacchetto::apri(Cursor::new(&self.byte)).ok();
        let xml = self
            .parte_stili
            .as_ref()
            .and_then(|p| pacchetto.as_mut()?.parte(p).ok().flatten())
            .unwrap_or_default();
        let dichiarazione = fine_dichiarazione(&xml);
        let mut corpo = xml[dichiarazione..].to_string();
        if corpo.contains("xmlns:w=") {
            if let Some(i) = corpo.rfind("</w:styles>") {
                let mut nuovi = String::new();
                for (livello, stile) in [(1, STILE_TITOLO_1), (2, STILE_TITOLO_2)] {
                    if self.stili.id_titolo(livello).is_none() {
                        nuovi.push_str(stile);
                    }
                }
                corpo.insert_str(i, &nuovi);
            }
        }
        format!("{INTESTAZIONE}\r\n{corpo}")
    }
}

/// Le proprietà di un run: quelle originali più ciò che i mark chiedono, solo dove
/// differisce da quello che già danno gli stili. In OOXML rStyle e rFonts vengono prima.
fn proprieta_run(formato: &str, r: &Run, (base_b, base_i): (bool, bool), (base_u, base_s): (bool, bool)) -> String {
    // una sottolineatura originale resta com'era finché il testo è sottolineato
    let originali: Vec<(String, &str)> = figli(formato)
        .into_iter()
        .filter(|(n, x)| match n.as_str() {
            "u" => r.sottolineato && attributo_di(x, "val").is_none_or(|v| v != "none"),
            "strike" | "dstrike" => r.barrato && !attributo_di(x, "val").is_some_and(|v| matches!(v.as_str(), "0" | "false" | "off")),
            _ => true,
        })
        .collect();
    let ha = |nomi: &[&str]| originali.iter().any(|(n, _)| nomi.contains(&n.as_str()));
    let (ha_u, ha_s) = (ha(&["u"]), ha(&["strike", "dstrike"]));
    let (prima, dopo): (Vec<_>, Vec<_>) = originali.iter().partition(|(n, _)| n == "rStyle" || n == "rFonts");

    let mut s = String::new();
    prima.iter().for_each(|(_, x)| s.push_str(x));
    if r.grassetto != base_b {
        s.push_str(if r.grassetto { "<w:b/>" } else { r#"<w:b w:val="0"/>"# });
    }
    if r.corsivo != base_i {
        s.push_str(if r.corsivo { "<w:i/>" } else { r#"<w:i w:val="0"/>"# });
    }
    if !ha_s && r.barrato != base_s {
        s.push_str(if r.barrato { "<w:strike/>" } else { r#"<w:strike w:val="0"/>"# });
    }
    dopo.iter().for_each(|(_, x)| s.push_str(x));
    if !ha_u && r.sottolineato != base_u {
        s.push_str(if r.sottolineato { r#"<w:u w:val="single"/>"# } else { r#"<w:u w:val="none"/>"# });
    }
    s
}

/// L'interno di <w:pPr> con un altro stile al posto del suo (w:pStyle deve restare il
/// primo figlio). Via anche un outlineLvl diretto, che farebbe il titolo o il testo al
/// posto dello stile.
fn con_stile(ppr: &str, stile: Option<&str>) -> String {
    let mut s = stile.map_or(String::new(), |id| format!(r#"<w:pStyle w:val="{id}"/>"#));
    figli(ppr).into_iter().filter(|(n, _)| n != "pStyle" && n != "outlineLvl").for_each(|(_, x)| s.push_str(x));
    s
}

/// L'interno di <w:pPr>, senza ciò che non va conservato.
pub(super) fn pulisci_ppr(interno: &str) -> String {
    let mut s = String::new();
    for (nome, xml) in figli(interno) {
        if VIA_DA_PPR.contains(&nome.as_str()) {
            continue;
        }
        if nome == "rPr" {
            // il segno di paragrafo: niente revisioni
            let dentro = xml.find('>').zip(xml.rfind("</")).filter(|(a, b)| a < b).map(|(a, b)| (a + 1, b));
            if let Some((a, b)) = dentro {
                let pulito = pulisci_rpr(&xml[a..b]);
                if !pulito.is_empty() {
                    s.push_str(&xml[..a]);
                    s.push_str(&pulito);
                    s.push_str(&xml[b..]);
                }
            }
            continue;
        }
        s.push_str(xml);
    }
    s
}

/// L'interno di <w:rPr>, senza grassetto e corsivo (stanno nei mark) e senza revisioni.
pub(super) fn pulisci_rpr(interno: &str) -> String {
    figli(interno).into_iter().filter(|(n, _)| !VIA_DA_RPR.contains(&n.as_str())).map(|(_, x)| x).collect()
}

/// I figli diretti di un frammento XML: (nome locale, testo grezzo dell'elemento).
fn figli(xml: &str) -> Vec<(String, &str)> {
    let mut reader = Reader::from_str(xml);
    let mut risultato = Vec::new();
    let (mut profondita, mut inizio, mut nome) = (0usize, 0usize, String::new());
    loop {
        let prima = reader.buffer_position() as usize;
        let Ok(evento) = reader.read_event() else { break };
        let dopo = reader.buffer_position() as usize;
        match evento {
            Event::Start(e) => {
                if profondita == 0 {
                    inizio = prima;
                    nome = e.local_name().as_ref().to_string();
                }
                profondita += 1;
            }
            Event::End(_) => {
                profondita = profondita.saturating_sub(1);
                if profondita == 0 {
                    risultato.push((nome.clone(), &xml[inizio..dopo]));
                }
            }
            Event::Empty(e) if profondita == 0 => risultato.push((e.local_name().as_ref().to_string(), &xml[prima..dopo])),
            Event::Eof => break,
            _ => {}
        }
    }
    risultato
}

/// w:val del figlio con quel nome (pStyle, rStyle).
fn valore_figlio(xml: &str, nome: &str) -> Option<String> {
    let (_, frammento) = figli(xml).into_iter().find(|(n, _)| n == nome)?;
    attributo_di(frammento, "val")
}

/// Un attributo dell'elemento con cui comincia il frammento.
fn attributo_di(frammento: &str, nome: &str) -> Option<String> {
    let mut reader = Reader::from_str(frammento);
    match reader.read_event().ok()? {
        Event::Start(e) | Event::Empty(e) => attributo(&e, nome),
        _ => None,
    }
}

/// Dove finisce <w:body> e dove comincia la coda: l'ultimo sectPr del corpo, se c'è,
/// altrimenti </w:body>.
fn corpo(xml: &str) -> Option<(usize, usize)> {
    let mut reader = Reader::from_str(xml);
    let mut profondita = 0usize;
    let (mut apertura, mut sezione, mut chiusura) = (None, None, None);
    loop {
        let prima = reader.buffer_position() as usize;
        let evento = reader.read_event().ok()?;
        let dopo = reader.buffer_position() as usize;
        match evento {
            Event::Start(e) => {
                let nome = e.local_name();
                if profondita == 1 && nome.as_ref() == "body" {
                    apertura = Some(dopo);
                } else if profondita == 2 && nome.as_ref() == "sectPr" {
                    sezione = Some(prima);
                }
                profondita += 1;
            }
            Event::Empty(e) if profondita == 2 && e.local_name().as_ref() == "sectPr" => sezione = Some(prima),
            Event::End(e) => {
                profondita = profondita.saturating_sub(1);
                if profondita == 1 && e.local_name().as_ref() == "body" {
                    chiusura = Some(prima);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Some((apertura?, sezione.or(chiusura)?))
}

/// L'inizio di document.xml, in UTF-8 e con il prefisso w: dichiarato, che usa il corpo.
fn apertura(xml: &str) -> Option<String> {
    let resto = &xml[fine_dichiarazione(xml)..];
    let mut s = format!("{INTESTAZIONE}\r\n{resto}");
    let radice = s.find("<w:").or_else(|| s[INTESTAZIONE.len()..].find('<').map(|i| i + INTESTAZIONE.len()))?;
    if !s.contains("xmlns:w=") {
        let fine = radice + s[radice..].find('>')?;
        s.insert_str(fine, r#" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#);
    }
    Some(s)
}

fn fine_dichiarazione(xml: &str) -> usize {
    let dopo = if xml.starts_with("<?xml") { xml.find("?>").map_or(0, |i| i + 2) } else { 0 };
    xml.len() - xml[dopo..].trim_start().len()
}
