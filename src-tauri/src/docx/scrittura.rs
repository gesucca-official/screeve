//! Scrittura: un docx minimo e valido, con gli stili integrati di Word.

use super::{Blocco, Run};
use std::io::{Seek, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;

const INTESTAZIONE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;
const NS_W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

const CONTENT_TYPES: &str = r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#;

const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

const DOCUMENT_RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/></Relationships>"#;

// Georgia 12 pt e lingua italiana per tutto il documento; paragrafi neutri (niente
// spaziatura né rientri, interlinea singola), l'impaginazione si fa poi in Word.
// Titolo e Sottotitolo sono gli stili integrati "heading 1" e "heading 2": Word li
// mostra come Titolo 1 e Titolo 2 e li mette nel riquadro di spostamento.
const STYLES: &str = concat!(
    r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
    r#"<w:docDefaults>"#,
    r#"<w:rPrDefault><w:rPr><w:rFonts w:ascii="Georgia" w:eastAsia="Georgia" w:hAnsi="Georgia" w:cs="Georgia"/><w:sz w:val="24"/><w:szCs w:val="24"/><w:lang w:val="it-IT" w:eastAsia="it-IT" w:bidi="ar-SA"/></w:rPr></w:rPrDefault>"#,
    r#"<w:pPrDefault><w:pPr><w:spacing w:before="0" w:after="0" w:line="240" w:lineRule="auto"/></w:pPr></w:pPrDefault>"#,
    r#"</w:docDefaults>"#,
    r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>"#,
    r#"<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:uiPriority w:val="9"/><w:qFormat/>"#,
    r#"<w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="480" w:after="120"/><w:outlineLvl w:val="0"/></w:pPr>"#,
    r#"<w:rPr><w:b/><w:bCs/><w:sz w:val="44"/><w:szCs w:val="44"/></w:rPr></w:style>"#,
    r#"<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:uiPriority w:val="9"/><w:unhideWhenUsed/><w:qFormat/>"#,
    r#"<w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="120" w:after="360"/><w:outlineLvl w:val="1"/></w:pPr>"#,
    r#"<w:rPr><w:i/><w:iCs/><w:sz w:val="28"/><w:szCs w:val="28"/></w:rPr></w:style>"#,
    r#"<w:style w:type="character" w:default="1" w:styleId="DefaultParagraphFont"><w:name w:val="Default Paragraph Font"/><w:uiPriority w:val="1"/><w:semiHidden/><w:unhideWhenUsed/></w:style>"#,
    r#"</w:styles>"#
);

// compatibilityMode 15: Word non apre il file in "Modalità compatibilità".
const SETTINGS: &str = r#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:zoom w:percent="100"/><w:defaultTabStop w:val="708"/><w:hyphenationZone w:val="283"/><w:characterSpacingControl w:val="doNotCompress"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat><w:themeFontLang w:val="it-IT"/><w:decimalSymbol w:val=","/><w:listSeparator w:val=";"/></w:settings>"#;

const APP: &str = r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Screeve</Application></Properties>"#;

/// Scrive il documento come .docx.
pub fn scrivi<W: Write + Seek>(destinazione: W, blocchi: &[Blocco]) -> zip::result::ZipResult<()> {
    let mut zip = ZipWriter::new(destinazione);
    let opzioni = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let parti: [(&str, String); 8] = [
        ("[Content_Types].xml", xml(CONTENT_TYPES)),
        ("_rels/.rels", xml(RELS)),
        ("word/document.xml", document(blocchi)),
        ("word/_rels/document.xml.rels", xml(DOCUMENT_RELS)),
        ("word/styles.xml", xml(STYLES)),
        ("word/settings.xml", xml(SETTINGS)),
        ("docProps/core.xml", core()),
        ("docProps/app.xml", xml(APP)),
    ];
    for (nome, contenuto) in parti {
        zip.start_file(nome, opzioni)?;
        zip.write_all(contenuto.as_bytes())?;
    }
    zip.finish()?;
    Ok(())
}

fn xml(corpo: &str) -> String {
    format!("{INTESTAZIONE}\r\n{corpo}")
}

pub(super) fn document(blocchi: &[Blocco]) -> String {
    let mut s = String::with_capacity(256 + blocchi.len() * 200);
    s.push_str(INTESTAZIONE);
    s.push_str("\r\n");
    s.push_str(&format!(r#"<w:document xmlns:w="{NS_W}"><w:body>"#));
    for b in blocchi {
        s.push_str("<w:p>");
        match b.livello_titolo() {
            1 => s.push_str(r#"<w:pPr><w:pStyle w:val="Heading1"/></w:pPr>"#),
            2 => s.push_str(r#"<w:pPr><w:pStyle w:val="Heading2"/></w:pPr>"#),
            _ => {}
        }
        for r in &b.run {
            run(&mut s, r);
        }
        s.push_str("</w:p>");
    }
    // A4 con i margini predefiniti della versione italiana di Word
    s.push_str(r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1417" w:right="1134" w:bottom="1134" w:left="1134" w:header="708" w:footer="708" w:gutter="0"/><w:cols w:space="708"/></w:sectPr>"#);
    s.push_str("</w:body></w:document>");
    s
}

fn run(s: &mut String, r: &Run) {
    if r.testo.is_empty() {
        return;
    }
    s.push_str("<w:r>");
    if r.grassetto || r.corsivo {
        s.push_str("<w:rPr>");
        if r.grassetto {
            s.push_str("<w:b/>");
        }
        if r.corsivo {
            s.push_str("<w:i/>");
        }
        s.push_str("</w:rPr>");
    }
    // tabulazioni e a capo hanno elementi propri, il resto va in <w:t>
    let mut pezzo = String::new();
    let chiudi = |s: &mut String, pezzo: &mut String| {
        if !pezzo.is_empty() {
            s.push_str(r#"<w:t xml:space="preserve">"#);
            s.push_str(pezzo);
            s.push_str("</w:t>");
            pezzo.clear();
        }
    };
    for c in r.testo.chars() {
        match c {
            '\t' => { chiudi(s, &mut pezzo); s.push_str("<w:tab/>"); }
            '\n' => { chiudi(s, &mut pezzo); s.push_str("<w:br/>"); }
            '&' => pezzo.push_str("&amp;"),
            '<' => pezzo.push_str("&lt;"),
            '>' => pezzo.push_str("&gt;"),
            // caratteri di controllo e non-caratteri non sono ammessi in XML
            c if c < ' ' || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => pezzo.push(c),
        }
    }
    chiudi(s, &mut pezzo);
    s.push_str("</w:r>");
}

fn core() -> String {
    xml(&format!(
        concat!(
            r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
            r#"<dc:language>it-IT</dc:language><dcterms:modified xsi:type="dcterms:W3CDTF">{}</dcterms:modified></cp:coreProperties>"#
        ),
        adesso_utc()
    ))
}

/// Data e ora attuali in UTC, nel formato W3CDTF (2026-10-30T09:15:00Z).
fn adesso_utc() -> String {
    let secondi = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (giorni, resto) = ((secondi / 86_400) as i64, secondi % 86_400);
    // da giorni dall'epoca a data civile (algoritmo di Howard Hinnant)
    let z = giorni + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let giorno = doy - (153 * mp + 2) / 5 + 1;
    let mese = if mp < 10 { mp + 3 } else { mp - 9 };
    let anno = yoe + era * 400 + i64::from(mese <= 2);
    format!(
        "{anno:04}-{mese:02}-{giorno:02}T{:02}:{:02}:{:02}Z",
        resto / 3600,
        resto % 3600 / 60,
        resto % 60
    )
}

#[cfg(test)]
pub(super) fn parti_fisse() -> [&'static str; 6] {
    [CONTENT_TYPES, RELS, DOCUMENT_RELS, STYLES, SETTINGS, APP]
}

#[cfg(test)]
pub(super) fn core_per_test() -> String {
    core()
}
