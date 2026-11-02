use super::*;
use std::io::Cursor;
use zip::write::{SimpleFileOptions, ZipWriter};

const W: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

fn p(testo: &str) -> Blocco {
    Blocco::paragrafo(vec![Run::new(testo, false, false)])
}

fn h(livello: u8, testo: &str) -> Blocco {
    Blocco::titolo(livello, vec![Run::new(testo, false, false)])
}

fn r(testo: &str, grassetto: bool, corsivo: bool) -> Run {
    Run::new(testo, grassetto, corsivo)
}

fn in_memoria(blocchi: &[Blocco]) -> Vec<u8> {
    let mut dati = Cursor::new(Vec::new());
    scrivi(&mut dati, blocchi).unwrap();
    dati.into_inner()
}

/// Un docx costruito a mano: document.xml, styles.xml e altre parti facoltative.
fn pacchetto(document: &str, styles: Option<&str>, altre: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opzioni = SimpleFileOptions::default();
    let mut aggiungi = |nome: &str, contenuto: &str| {
        zip.start_file(nome, opzioni).unwrap();
        zip.write_all(contenuto.as_bytes()).unwrap();
    };
    aggiungi("[Content_Types].xml", r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#);
    aggiungi("_rels/.rels", r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="/word/document.xml"/></Relationships>"#);
    let mut rels = String::from(r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
    if styles.is_some() {
        rels.push_str(r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#);
    }
    for (i, (nome, _)) in altre.iter().enumerate() {
        let tipo = if nome.contains("header") { "header" } else if nome.contains("footer") { "footer" } else { "comments" };
        rels.push_str(&format!(
            r#"<Relationship Id="rIdx{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/{tipo}" Target="{}"/>"#,
            nome.trim_start_matches("word/")
        ));
    }
    rels.push_str(r#"<Relationship Id="rIdL" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/></Relationships>"#);
    aggiungi("word/_rels/document.xml.rels", &rels);
    aggiungi("word/document.xml", document);
    if let Some(s) = styles {
        aggiungi("word/styles.xml", s);
    }
    for (nome, contenuto) in altre {
        aggiungi(nome, contenuto);
    }
    zip.finish().unwrap().into_inner()
}

/// Il formato originale ha i suoi test (modello_*): gli altri guardano testo e struttura.
fn senza_formato(mut d: Documento) -> Documento {
    for b in &mut d.blocchi {
        b.formato = None;
        // run distinti solo per il font tornano uno
        let mut run: Vec<Run> = Vec::new();
        for r in std::mem::take(&mut b.run) {
            match run.last_mut() {
                Some(u) if u.stesso_aspetto(&r) => u.testo.push_str(&r.testo),
                _ => run.push(Run { formato: None, ..r }),
            }
        }
        b.run = run;
    }
    d
}

fn leggi_byte(dati: Vec<u8>) -> Documento {
    senza_formato(leggi(Cursor::new(dati)).unwrap())
}

fn non_supportati(d: &Documento) -> Vec<(&'static str, u32)> {
    d.non_supportati.iter().map(|n| (n.elemento, n.quanti)).collect()
}

fn esempio() -> Vec<Blocco> {
    vec![
        h(1, "Il guardiano del faro"),
        h(2, "Capitolo primo, in cui si sale"),
        Blocco::paragrafo(vec![
            r("Ogni sera, prima che il sole toccasse il mare, Elia saliva i ", false, false),
            r("centotrentadue", true, false),
            r(" gradini del faro contandoli a voce bassa. Non lo faceva per abitudine, ma ", false, false),
            r("perché", false, true),
            r(" temeva che un giorno ne avrebbe trovato uno in più.", false, false),
        ]),
        Blocco::paragrafo(vec![
            r("«È tardi» disse la madre. «", false, false),
            r("Sempre", true, true),
            r(" tardi.»", false, false),
        ]),
        p("‹Così› pensò, e l'acqua dell'insenatura gli parve più scura: città, virtù, perché, poiché."),
        Blocco::paragrafo(vec![]),
        h(1, "Capitolo secondo & <ultimo>"),
        p("Colonna\tcon tabulazione, \"virgolette\" e 'apostrofi' — lineetta… fine."),
    ]
}

/* ---------- scrittura e round-trip ---------- */

#[test]
fn round_trip() {
    let blocchi = esempio();
    let letto = leggi_byte(in_memoria(&blocchi));
    assert_eq!(letto.blocchi, blocchi);
    assert!(letto.non_supportati.is_empty(), "{:?}", letto.non_supportati);
}

#[test]
fn round_trip_documento_vuoto() {
    let letto = leggi_byte(in_memoria(&[]));
    assert!(letto.blocchi.is_empty());
    let letto = leggi_byte(in_memoria(&[Blocco::paragrafo(vec![])]));
    assert_eq!(letto.blocchi, vec![Blocco::paragrafo(vec![])]);
}

#[test]
fn parti_ben_formate() {
    let mut parti: Vec<String> = scrittura::parti_fisse().iter().map(|s| s.to_string()).collect();
    parti.push(scrittura::document(&esempio()));
    parti.push(scrittura::core_per_test());
    for xml in parti {
        let mut reader = quick_xml::Reader::from_str(&xml);
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => panic!("XML non valido: {e}\n{xml}"),
            }
        }
    }
}

#[test]
fn stili_e_formattazione() {
    let document = scrittura::document(&esempio());
    assert!(document.contains(r#"<w:pStyle w:val="Heading1"/>"#));
    assert!(document.contains(r#"<w:pStyle w:val="Heading2"/>"#));
    assert!(document.contains("<w:rPr><w:b/><w:i/></w:rPr>"));
    assert!(document.contains("Capitolo secondo &amp; &lt;ultimo&gt;"));
    assert!(document.contains("Colonna</w:t><w:tab/><w:t"));
    let [_, _, _, styles, settings, _] = scrittura::parti_fisse();
    assert!(styles.contains(r#"w:ascii="Calibri""#));
    assert!(styles.contains(r#"<w:sz w:val="24"/>"#));
    assert!(styles.contains(r#"<w:lang w:val="it-IT""#));
    assert!(styles.contains(r#"<w:name w:val="heading 1"/>"#));
    assert!(styles.contains(r#"<w:name w:val="heading 2"/>"#));
    assert!(styles.contains(r#"w:before="0" w:after="160" w:line="300""#));
    assert!(!styles.contains("w:firstLine"));
    assert!(settings.contains(r#"w:val="15""#));
}

#[test]
fn caratteri_non_ammessi_in_xml() {
    let blocchi = [Blocco::paragrafo(vec![r("a\u{1}b\u{FFFF}c", false, false)])];
    let letto = leggi_byte(in_memoria(&blocchi));
    assert_eq!(letto.blocchi, vec![p("abc")]);
}

#[test]
fn salvataggio_atomico() {
    let cartella = std::env::temp_dir().join(format!("screeve-test-{}", std::process::id()));
    fs::create_dir_all(&cartella).unwrap();
    let file = cartella.join("Il guardiano del faro.docx");

    salva(&file, &esempio(), None).unwrap();
    salva(&file, &[p("Seconda versione")], None).unwrap(); // sovrascrive
    assert_eq!(senza_formato(apri(&file).unwrap().0).blocchi, vec![p("Seconda versione")]);

    let rimasti: Vec<_> = fs::read_dir(&cartella).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(rimasti.len(), 1, "temporanei rimasti: {rimasti:?}");

    // cartella inesistente: errore chiaro, e nessun file a metà
    let errore = salva(&cartella.join("non/esiste.docx"), &esempio(), None).unwrap_err();
    assert!(errore.contains("esiste.docx"), "{errore}");
    fs::remove_dir_all(&cartella).unwrap();
}

#[test]
fn file_non_docx() {
    let errore = leggi(Cursor::new(b"{\\rtf1 un vecchio documento}".to_vec())).unwrap_err();
    assert!(errore.contains(".docx"), "{errore}");
    let errore = apri(Path::new("Z:/non/esiste.docx")).err().unwrap();
    assert!(errore.contains("esiste.docx"), "{errore}");
}

/* ---------- lettura di file prodotti da altri programmi ---------- */

// Word in italiano: id degli stili localizzati, nomi interni in inglese.
const STILI_WORD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="w14">
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:asciiTheme="minorHAnsi"/><w:lang w:val="it-IT"/></w:rPr></w:rPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normale"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Titolo1"><w:name w:val="heading 1"/><w:basedOn w:val="Normale"/><w:next w:val="Normale"/><w:link w:val="Titolo1Carattere"/><w:pPr><w:keepNext/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:rFonts w:asciiTheme="majorHAnsi"/><w:color w:val="0F4761"/><w:sz w:val="40"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Titolo2"><w:name w:val="heading 2"/><w:basedOn w:val="Normale"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Titolo3"><w:name w:val="heading 3"/><w:basedOn w:val="Normale"/><w:pPr><w:outlineLvl w:val="2"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Titolo"><w:name w:val="Title"/><w:basedOn w:val="Normale"/><w:rPr><w:b/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Sottotitolo"><w:name w:val="Subtitle"/><w:basedOn w:val="Normale"/><w:rPr><w:i/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Capitolo"><w:name w:val="Capitolo"/><w:basedOn w:val="Titolo1"/></w:style>
<w:style w:type="paragraph" w:styleId="Citazione"><w:name w:val="Quote"/><w:basedOn w:val="Normale"/><w:rPr><w:i/><w:iCs/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Paragrafoelenco"><w:name w:val="List Paragraph"/><w:basedOn w:val="Normale"/></w:style>
<w:style w:type="character" w:default="1" w:styleId="Carpredefinitoparagrafo"><w:name w:val="Default Paragraph Font"/></w:style>
<w:style w:type="character" w:styleId="Enfasigrassetto"><w:name w:val="Strong"/><w:rPr><w:b/><w:bCs/></w:rPr></w:style>
<w:style w:type="character" w:styleId="Enfasicorsivo"><w:name w:val="Emphasis"/><w:rPr><w:i/><w:iCs/></w:rPr></w:style>
<w:style w:type="character" w:styleId="Titolo1Carattere"><w:name w:val="Titolo 1 Carattere"/><w:link w:val="Titolo1"/><w:rPr><w:b/></w:rPr></w:style>
</w:styles>"#;

#[test]
fn word_titoli_localizzati_e_formattazione() {
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {W} xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="w14"><w:body>
<w:p w14:paraId="1A2B3C4D" w:rsidR="00A1"><w:pPr><w:pStyle w:val="Titolo"/></w:pPr><w:r><w:t>Il libro</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Sottotitolo"/></w:pPr><w:r><w:t>Un romanzo</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Titolo1"/><w:rPr><w:b/></w:rPr></w:pPr><w:bookmarkStart w:id="0" w:name="_Toc1"/><w:r><w:t>Capitolo uno</w:t></w:r><w:bookmarkEnd w:id="0"/></w:p>
<w:p><w:pPr><w:pStyle w:val="Titolo3"/></w:pPr><w:r><w:t xml:space="preserve">Una sezione</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Capitolo"/></w:pPr><w:r><w:t>Capitolo due</w:t></w:r></w:p>
<w:p w:rsidR="00B2"><w:r><w:t xml:space="preserve">Era </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>notte</w:t></w:r><w:proofErr w:type="spellStart"/><w:r><w:rPr><w:i/></w:rPr><w:t xml:space="preserve"> fonda</w:t></w:r><w:proofErr w:type="spellEnd"/><w:r><w:rPr><w:rStyle w:val="Enfasigrassetto"/></w:rPr><w:t xml:space="preserve">, </w:t></w:r><w:r><w:rPr><w:rStyle w:val="Enfasigrassetto"/><w:b w:val="0"/></w:rPr><w:t>spento</w:t></w:r><w:r><w:rPr><w:i w:val="false"/></w:rPr><w:t xml:space="preserve"> e </w:t></w:r><w:r><w:rPr><w:rStyle w:val="Enfasicorsivo"/></w:rPr><w:t>enfasi</w:t></w:r><w:r><w:lastRenderedPageBreak/><w:t>.</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Citazione"/></w:pPr><w:r><w:t xml:space="preserve">Una citazione </w:t></w:r><w:r><w:rPr><w:rStyle w:val="Enfasicorsivo"/></w:rPr><w:t>dritta</w:t></w:r></w:p>
<w:p><w:r><w:tab/><w:t>Rientro</w:t></w:r><w:r><w:tab/><w:t>tab</w:t></w:r><w:r><w:br/><w:t>riga due</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Titolo1"/></w:pPr><w:r><w:t>Titolo</w:t></w:r><w:r><w:br/><w:t>su due righe</w:t></w:r></w:p>
<w:p><w:r><w:t>Pagina </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>3</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:r><w:t xml:space="preserve"> di </w:t></w:r><w:fldSimple w:instr=" NUMPAGES "><w:r><w:t>9</w:t></w:r></w:fldSimple></w:p>
<w:p><w:r><w:t>A&amp;B &#171;ok&#187; &lt;3</w:t></w:r></w:p>
<w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr>
</w:body></w:document>"#
    );
    let d = leggi_byte(pacchetto(&document, Some(STILI_WORD), &[]));
    assert_eq!(
        d.blocchi,
        vec![
            h(1, "Il libro"),
            h(2, "Un romanzo"),
            h(1, "Capitolo uno"),
            h(2, "Una sezione"),
            h(1, "Capitolo due"),
            Blocco::paragrafo(vec![
                r("Era ", false, false),
                r("notte", true, false),
                r(" fonda", false, true),
                r(", ", true, false),
                r("spento e ", false, false),
                r("enfasi", false, true),
                r(".", false, false),
            ]),
            // lo stile Citazione è corsivo; il carattere Enfasi corsivo lo annulla, come in Word
            Blocco::paragrafo(vec![r("Una citazione ", false, true), r("dritta", false, false)]),
            Blocco::paragrafo(vec![r("Rientro\ttab", false, false)]),
            p("riga due"),
            h(1, "Titolo su due righe"),
            p("Pagina 3 di 9"),
            p("A&B «ok» <3"),
        ]
    );
    assert_eq!(non_supportati(&d), vec![("campi", 2)]);
}

#[test]
fn word_elementi_non_supportati() {
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {W} xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>
<w:p><w:r><w:t>Prima</w:t></w:r><w:r><w:drawing><wp:inline><a:graphic><a:graphicData><a:p><a:r><a:t>non io</a:t></a:r></a:p></a:graphicData></a:graphic></wp:inline></w:drawing></w:r><w:r><w:t xml:space="preserve"> dopo</w:t></w:r></w:p>
<w:p><w:r><mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wp:anchor><a:graphic><a:graphicData><wps:wsp><wps:txbx><w:txbxContent><w:p><w:r><w:t>casella</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></a:graphicData></a:graphic></wp:anchor></w:drawing></mc:Choice><mc:Fallback><w:pict><v:shape><v:textbox><w:txbxContent><w:p><w:r><w:t>casella</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p>
<w:tbl><w:tblPr/><w:tr><w:tc><w:p><w:r><w:t>Cella uno</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Annidata</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:tc><w:tc><w:p><w:r><w:t>Cella due</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:t>Nota</w:t></w:r><w:r><w:rPr><w:rStyle w:val="Rimandonotaapidipagina"/></w:rPr><w:footnoteReference w:id="1"/></w:r><w:commentRangeStart w:id="0"/><w:r><w:t xml:space="preserve"> commentata</w:t></w:r><w:commentRangeEnd w:id="0"/><w:r><w:commentReference w:id="0"/></w:r></w:p>
<w:p><w:r><w:t xml:space="preserve">Revisione: </w:t></w:r><w:del w:id="1" w:author="A"><w:r><w:delText>vecchio</w:delText></w:r></w:del><w:ins w:id="2" w:author="A"><w:r><w:t>nuovo</w:t></w:r></w:ins><w:moveFrom w:id="3"><w:r><w:t>spostato</w:t></w:r></w:moveFrom><w:r><w:rPr><w:b/><w:rPrChange w:id="4"><w:rPr><w:i/></w:rPr></w:rPrChange></w:rPr><w:t xml:space="preserve"> grassetto</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Paragrafoelenco"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Voce uno</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Paragrafoelenco"/><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Voce due</w:t></w:r></w:p>
<w:p><w:pPr><w:numPr><w:numId w:val="0"/></w:numPr></w:pPr><w:r><w:t>Non in elenco</w:t></w:r></w:p>
<w:p><w:r><w:br w:type="page"/></w:r><w:hyperlink r:id="rIdL"><w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>sito</w:t></w:r></w:hyperlink><w:r><w:t xml:space="preserve"> e </w:t></w:r><w:r><w:rPr><w:u w:val="none"/></w:rPr><w:t>no</w:t></w:r><w:r><w:sym w:font="Wingdings" w:char="F04A"/></w:r></w:p>
<w:p><m:oMathPara><m:oMath><m:r><m:t>x=1</m:t></m:r></m:oMath></m:oMathPara></w:p>
<w:p><w:pPr><w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:pPr><w:r><w:t>Fine sezione</w:t></w:r></w:p>
<w:sectPr><w:headerReference w:type="default" r:id="rIdx0"/></w:sectPr>
</w:body></w:document>"#
    );
    let intestazione = format!(r#"<w:hdr {W}><w:p><w:r><w:t>Il guardiano del faro</w:t></w:r></w:p></w:hdr>"#);
    let pie_vuoto = format!(r#"<w:ftr {W}><w:p/></w:ftr>"#);
    let d = leggi_byte(pacchetto(
        &document,
        Some(STILI_WORD),
        &[("word/header1.xml", &intestazione), ("word/footer1.xml", &pie_vuoto)],
    ));
    assert_eq!(
        d.blocchi,
        vec![
            p("Prima dopo"),
            Blocco::paragrafo(vec![]),
            p("Cella uno"),
            p("Annidata"),
            p("Cella due"),
            p("Nota commentata"),
            Blocco::paragrafo(vec![r("Revisione: nuovo", false, false), r(" grassetto", true, false)]),
            p("Voce uno"),
            p("Voce due"),
            p("Non in elenco"),
            Blocco::paragrafo(vec![Run { sottolineato: true, ..r("sito", false, false) }, r(" e no", false, false)]),
            Blocco::paragrafo(vec![]),
            p("Fine sezione"),
        ]
    );
    assert_eq!(
        non_supportati(&d),
        vec![
            ("tabelle", 1),
            ("immagini", 1),
            ("caselle", 1),
            ("formule", 1),
            ("note", 1),
            ("commenti", 1),
            ("revisioni", 4),
            ("elenchi", 2),
            ("interruzioni", 2),
            ("collegamenti", 1),
            ("simboli", 1),
        ]
    );
}

// Google Documenti: id inglesi, w:val="1" e "0" espliciti, Title/Subtitle e Heading3.
#[test]
fn google_documenti() {
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles {W}><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:sz w:val="22"/><w:lang w:val="it"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:line="276" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext w:val="1"/><w:spacing w:before="400" w:after="120"/></w:pPr><w:rPr><w:sz w:val="40"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:rPr><w:sz w:val="32"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:rPr><w:sz w:val="52"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/><w:basedOn w:val="Normal"/><w:rPr><w:i w:val="0"/><w:color w:val="666666"/></w:rPr></w:style>
</w:styles>"#
    );
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document {W}><w:body>
<w:p w:rsidR="0000" w:rsidDel="0000" w:rsidP="0000" w:rsidRDefault="0000"><w:pPr><w:pStyle w:val="Title"/><w:rPr/></w:pPr><w:r w:rsidDel="0000" w:rsidR="0000" w:rsidRPr="0000"><w:rPr><w:rtl w:val="0"/></w:rPr><w:t xml:space="preserve">La città</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Subtitle"/></w:pPr><w:r><w:rPr><w:rtl w:val="0"/></w:rPr><w:t xml:space="preserve">Racconto</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:bookmarkStart w:colFirst="0" w:colLast="0" w:name="_heading=h.1" w:id="0"/><w:bookmarkEnd w:id="0"/><w:r><w:rPr><w:rtl w:val="0"/></w:rPr><w:t xml:space="preserve">Uno</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading3"/></w:pPr><w:r><w:t xml:space="preserve">Parte</w:t></w:r></w:p>
<w:p><w:pPr><w:rPr/></w:pPr><w:r><w:rPr><w:b w:val="1"/><w:rtl w:val="0"/></w:rPr><w:t xml:space="preserve">Forte</w:t></w:r><w:r><w:rPr><w:b w:val="0"/><w:i w:val="1"/><w:rtl w:val="0"/></w:rPr><w:t xml:space="preserve"> e lieve, </w:t></w:r><w:r><w:rPr><w:b w:val="1"/><w:i w:val="1"/></w:rPr><w:t xml:space="preserve">entrambi</w:t></w:r><w:r><w:rPr><w:b w:val="0"/><w:i w:val="0"/></w:rPr><w:t xml:space="preserve">.</w:t></w:r></w:p>
<w:p><w:pPr><w:rPr/></w:pPr><w:r><w:rPr><w:rtl w:val="0"/></w:rPr></w:r></w:p>
<w:sectPr><w:pgSz w:h="16834" w:w="11909" w:orient="portrait"/></w:sectPr>
</w:body></w:document>"#
    );
    let d = leggi_byte(pacchetto(&document, Some(&styles), &[]));
    assert_eq!(
        d.blocchi,
        vec![
            h(1, "La città"),
            h(2, "Racconto"),
            h(1, "Uno"),
            h(2, "Parte"),
            Blocco::paragrafo(vec![
                r("Forte", true, false),
                r(" e lieve, ", false, true),
                r("entrambi", true, true),
                r(".", false, false),
            ]),
            Blocco::paragrafo(vec![]),
        ]
    );
    assert!(d.non_supportati.is_empty());
}

// LibreOffice: titoli in grassetto nello stile, stili di paragrafo personalizzati,
// outlineLvl diretto, stile senza nome e nessuna dichiarazione di stile predefinito.
#[test]
fn libreoffice() {
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:styles {W}>
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Liberation Serif" w:hAnsi="Liberation Serif"/><w:lang w:val="it-IT"/></w:rPr></w:rPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
<w:style w:type="paragraph" w:styleId="Heading"><w:name w:val="Heading"/><w:basedOn w:val="Normal"/><w:next w:val="TextBody"/><w:rPr><w:rFonts w:ascii="Liberation Sans"/><w:sz w:val="28"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Heading"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:bCs/><w:sz w:val="36"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:basedOn w:val="Heading"/><w:pPr><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:i/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="TextBody"><w:name w:val="Body Text"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="140"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Lettera"><w:name w:val="Lettera"/><w:basedOn w:val="TextBody"/><w:rPr><w:i/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Corpodeltesto"><w:name w:val="Corpo del testo"/><w:basedOn w:val="Normal"/><w:pPr><w:outlineLvl w:val="9"/></w:pPr></w:style>
<w:style w:type="character" w:styleId="StrongEmphasis"><w:name w:val="Strong"/><w:rPr><w:b/><w:bCs/></w:rPr></w:style>
</w:styles>"#
    );
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document {W}><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading1"/><w:rPr></w:rPr></w:pPr><w:r><w:rPr></w:rPr><w:t>Primo</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t xml:space="preserve">Sotto </w:t></w:r><w:r><w:rPr><w:rStyle w:val="StrongEmphasis"/></w:rPr><w:t>forte</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="TextBody"/></w:pPr><w:r><w:t xml:space="preserve">Testo </w:t></w:r><w:r><w:rPr><w:rStyle w:val="StrongEmphasis"/></w:rPr><w:t>forte</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Lettera"/></w:pPr><w:r><w:t xml:space="preserve">Cara Anna, </w:t></w:r><w:r><w:rPr><w:i w:val="false"/></w:rPr><w:t>dritto</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="TextBody"/><w:outlineLvl w:val="0"/></w:pPr><w:r><w:t>Livello diretto</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Corpodeltesto"/></w:pPr><w:r><w:t>Corpo</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="StileCheNonEsiste"/></w:pPr><w:r><w:t>Sconosciuto</w:t></w:r></w:p>
<w:p><w:r><w:t>Senza stile</w:t></w:r></w:p>
<w:sectPr/></w:body></w:document>"#
    );
    let d = leggi_byte(pacchetto(&document, Some(&styles), &[]));
    assert_eq!(
        d.blocchi,
        vec![
            h(1, "Primo"),
            Blocco::titolo(2, vec![r("Sotto ", false, false), r("forte", true, false)]),
            Blocco::paragrafo(vec![r("Testo ", false, false), r("forte", true, false)]),
            Blocco::paragrafo(vec![r("Cara Anna, ", false, true), r("dritto", false, false)]),
            h(1, "Livello diretto"),
            p("Corpo"),
            p("Sconosciuto"),
            p("Senza stile"),
        ]
    );
    assert!(d.non_supportati.is_empty());
}

#[test]
fn senza_stili_e_con_bom() {
    let document = format!(
        "\u{FEFF}<?xml version=\"1.0\"?><w:document {W}><w:body><w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Titolo</w:t></w:r></w:p><w:p><w:r><w:t>Testo</w:t></w:r></w:p></w:body></w:document>"
    );
    // senza styles.xml l'id dello stile è l'unico indizio, e basta
    let d = leggi_byte(pacchetto(&document, None, &[]));
    assert_eq!(d.blocchi, vec![h(1, "Titolo"), p("Testo")]);
}

#[test]
fn livelli_dai_nomi() {
    use super::lettura::livello_da_nome as l;
    assert_eq!(l("heading 1"), Some(1));
    assert_eq!(l("Heading 4"), Some(2));
    assert_eq!(l("Titolo 1"), Some(1));
    assert_eq!(l("titolo  2"), Some(2));
    assert_eq!(l("Title"), Some(1));
    assert_eq!(l("Sottotitolo"), Some(2));
    assert_eq!(l("Heading"), None);
    assert_eq!(l("Normal"), None);
    assert_eq!(l("heading1"), Some(1));
}

/// Il file di prova da aprire a mano in Word e Google Documenti:
/// `cargo test -- --ignored genera_esempio` lo rigenera in docs/esempio.docx.
#[test]
#[ignore]
fn genera_esempio() {
    let mut blocchi = esempio();
    blocchi.retain(|b| !b.run.iter().any(|r| r.testo.contains('<')));
    blocchi.extend([
        h(1, "Capitolo secondo"),
        h(2, "Dove si scende"),
        Blocco::paragrafo(vec![
            r("Il ", false, false),
            r("grassetto", true, false),
            r(", il ", false, false),
            r("corsivo", false, true),
            r(" e il ", false, false),
            r("grassetto corsivo", true, true),
            r(" devono restare tali.", false, false),
        ]),
        p("Accenti: à è é ì ò ù À È É Ì Ò Ù — perché, né, sé, più, già, ciò, così."),
        p("Caporali: «Vieni» disse. ‹Forse› pensò lei. Apostrofi: dell'acqua, l’onda, po'."),
        Blocco::paragrafo(vec![r("E l'ultima riga, tutta in corsivo.", false, true)]),
    ]);
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/esempio.docx");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    salva(&file, &blocchi, None).unwrap();
    assert_eq!(senza_formato(apri(&file).unwrap().0).blocchi, blocchi);
}

/* ---------- il file aperto come modello ---------- */

fn parte_zip(dati: &[u8], nome: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(dati)).unwrap();
    let mut testo = String::new();
    std::io::Read::read_to_string(&mut zip.by_name(nome).unwrap(), &mut testo).unwrap();
    testo
}

fn risalva(originale: Vec<u8>, modifica: impl FnOnce(&mut Vec<Blocco>)) -> Vec<u8> {
    let mut d = leggi(Cursor::new(&originale)).unwrap();
    modifica(&mut d.blocchi);
    let modello = Modello::nuovo(originale).unwrap();
    let mut dati = Cursor::new(Vec::new());
    modello.scrivi(&mut dati, &d.blocchi).unwrap();
    dati.into_inner()
}

const STILI_MODELLO: &str = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normale"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Titolo1"><w:name w:val="heading 1"/><w:basedOn w:val="Normale"/></w:style><w:style w:type="paragraph" w:styleId="Citazione"><w:name w:val="Quote"/><w:rPr><w:i/></w:rPr></w:style></w:styles>"#;

fn documento_impaginato() -> Vec<u8> {
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document {W} xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:background w:color="FFFFEE"/><w:body>
<w:p><w:pPr><w:pStyle w:val="Titolo1"/><w:jc w:val="center"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Verdana"/><w:color w:val="112233"/></w:rPr><w:t>Il faro</w:t></w:r></w:p>
<w:p><w:pPr><w:spacing w:after="240" w:line="360" w:lineRule="auto"/><w:ind w:firstLine="284"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:b/><w:sz w:val="22"/></w:rPr><w:t>Forte</w:t></w:r><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:sz w:val="22"/></w:rPr><w:t xml:space="preserve"> e piano</w:t></w:r></w:p>
<w:p><w:pPr><w:pStyle w:val="Citazione"/></w:pPr><w:r><w:t>Una citazione</w:t></w:r></w:p>
<w:sectPr><w:headerReference w:type="default" r:id="rIdx0"/><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="2000" w:right="1800" w:bottom="2000" w:left="1800" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr>
</w:body></w:document>"#
    );
    let intestazione = format!(r#"<w:hdr {W}><w:p><w:r><w:t>Intestazione</w:t></w:r></w:p></w:hdr>"#);
    pacchetto(&document, Some(STILI_MODELLO), &[("word/header1.xml", &intestazione)])
}

#[test]
fn modello_conserva_impaginazione() {
    let originale = documento_impaginato();
    let dati = risalva(originale.clone(), |b| {
        b[1].run[1].testo = " e riscritto".into();
    });
    let xml = parte_zip(&dati, "word/document.xml");
    // margini, sfondo, spazi dei nomi
    assert!(xml.contains(r#"<w:pgMar w:top="2000" w:right="1800""#), "{xml}");
    assert!(xml.contains(r#"<w:background w:color="FFFFEE"/>"#), "{xml}");
    assert!(xml.contains("xmlns:w14="), "{xml}");
    assert!(xml.contains(r#"w:type="default" r:id="rIdx0""#), "{xml}");
    // paragrafo: interlinea, spazio, rientro, stile e allineamento
    assert!(xml.contains(r#"<w:pPr><w:spacing w:after="240" w:line="360" w:lineRule="auto"/><w:ind w:firstLine="284"/></w:pPr>"#), "{xml}");
    assert!(xml.contains(r#"<w:pPr><w:pStyle w:val="Titolo1"/><w:jc w:val="center"/></w:pPr>"#), "{xml}");
    // run: font e corpo, col grassetto al posto giusto e il testo nuovo
    assert!(xml.contains(r#"<w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:b/><w:sz w:val="22"/></w:rPr><w:t xml:space="preserve">Forte"#), "{xml}");
    assert!(xml.contains("e riscritto"), "{xml}");
    assert!(xml.contains(r#"<w:rFonts w:ascii="Verdana"/><w:color w:val="112233"/>"#), "{xml}");
    // le altre parti restano identiche
    for parte in ["word/styles.xml", "word/header1.xml", "word/_rels/document.xml.rels", "[Content_Types].xml"] {
        assert_eq!(parte_zip(&dati, parte), parte_zip(&originale, parte), "{parte}");
    }
}

#[test]
fn modello_risalvare_non_cambia_nulla() {
    let una = risalva(documento_impaginato(), |_| {});
    let due = risalva(una.clone(), |_| {});
    assert_eq!(parte_zip(&una, "word/document.xml"), parte_zip(&due, "word/document.xml"));
    assert_eq!(leggi_byte(una).blocchi, leggi_byte(due).blocchi);
}

#[test]
fn modello_paragrafo_nuovo_eredita() {
    let dati = risalva(documento_impaginato(), |b| {
        b.insert(2, p("Scritto adesso"));
    });
    let xml = parte_zip(&dati, "word/document.xml");
    let i = xml.find("Scritto adesso").unwrap();
    let inizio = xml[..i].rfind("<w:p>").unwrap();
    let nuovo = &xml[inizio..i];
    assert!(nuovo.contains(r#"<w:spacing w:after="240" w:line="360""#), "{nuovo}");
    assert!(nuovo.contains(r#"<w:rFonts w:ascii="Arial""#), "{nuovo}");
    assert!(!nuovo.contains("<w:b/>"), "{nuovo}");
}

#[test]
fn modello_titolo_nuovo_usa_lo_stile_del_file() {
    let dati = risalva(documento_impaginato(), |b| b.push(h(1, "Capitolo nuovo")));
    let xml = parte_zip(&dati, "word/document.xml");
    assert!(xml.contains(r#"<w:pStyle w:val="Titolo1"/></w:pPr><w:r><w:t xml:space="preserve">Capitolo nuovo"#), "{xml}");
    assert_eq!(parte_zip(&dati, "word/styles.xml"), STILI_MODELLO);

    // il file non ha sottotitoli: lo stile si aggiunge, senza toccare gli altri
    let dati = risalva(documento_impaginato(), |b| b.push(h(2, "Sottotitolo nuovo")));
    let xml = parte_zip(&dati, "word/document.xml");
    assert!(xml.contains(r#"<w:pStyle w:val="Heading2"/>"#), "{xml}");
    let stili = parte_zip(&dati, "word/styles.xml");
    assert!(stili.contains(r#"w:styleId="Heading2""#), "{stili}");
    assert!(stili.contains(r#"w:styleId="Citazione""#), "{stili}");
    assert_eq!(leggi_byte(dati).blocchi.last().unwrap().livello, 2);
}

#[test]
fn modello_togliere_il_corsivo_di_uno_stile() {
    let originale = documento_impaginato();
    assert!(leggi_byte(originale.clone()).blocchi[2].run[0].corsivo);
    let dati = risalva(originale, |b| b[2].run[0].corsivo = false);
    let xml = parte_zip(&dati, "word/document.xml");
    assert!(xml.contains(r#"<w:i w:val="0"/>"#), "{xml}");
    assert!(!leggi_byte(dati).blocchi[2].run[0].corsivo);
    // se resta corsivo basta lo stile: niente marcatura in più
    let xml = parte_zip(&risalva(documento_impaginato(), |_| {}), "word/document.xml");
    assert!(!xml.contains("<w:i/>") && !xml.contains("<w:i "), "{xml}");
}

#[test]
fn modello_non_conserva_cio_che_dichiara_perso() {
    let document = format!(
        r#"<w:document {W}><w:body><w:p><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr><w:pageBreakBefore/><w:sectPr><w:pgSz w:w="1"/></w:sectPr><w:jc w:val="right"/><w:rPr><w:ins w:id="1"/><w:color w:val="FF0000"/></w:rPr></w:pPr><w:r><w:rPr><w:u w:val="single"/><w:color w:val="00FF00"/></w:rPr><w:t>Voce</w:t></w:r></w:p><w:sectPr><w:pgSz w:w="11906"/></w:sectPr></w:body></w:document>"#
    );
    let xml = parte_zip(&risalva(pacchetto(&document, None, &[]), |_| {}), "word/document.xml");
    for via in ["numPr", "pageBreakBefore", r#"w:w="1""#, "w:ins"] {
        assert!(!xml.contains(via), "{via}: {xml}");
    }
    assert!(xml.contains(r#"<w:jc w:val="right"/><w:rPr><w:color w:val="FF0000"/></w:rPr>"#), "{xml}");
    assert!(xml.contains(r#"<w:rPr><w:u w:val="single"/><w:color w:val="00FF00"/></w:rPr>"#), "{xml}");
}

#[test]
fn modello_senza_sezione_e_con_altro_prefisso() {
    // niente sectPr, documento in UTF-16 dichiarato: si riscrive in UTF-8
    let document = r#"<?xml version="1.0" encoding="UTF-16"?><d:document xmlns:d="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><d:body><d:p><d:r><d:t>Altro prefisso</d:t></d:r></d:p></d:body></d:document>"#;
    let dati = risalva(pacchetto(document, None, &[]), |b| b.push(p("Aggiunto")));
    let xml = parte_zip(&dati, "word/document.xml");
    assert!(xml.contains(r#"encoding="UTF-8""#), "{xml}");
    assert_eq!(leggi_byte(dati).blocchi, vec![p("Altro prefisso"), p("Aggiunto")]);
}

#[test]
fn file_nuovo_calibri_con_aria() {
    let stili = parte_zip(&in_memoria(&esempio()), "word/styles.xml");
    assert!(stili.contains(r#"w:ascii="Calibri""#), "{stili}");
    assert!(stili.contains(r#"w:after="160" w:line="300""#), "{stili}");
}

/* ---------- sottolineato e barrato ---------- */

fn decorato(testo: &str, sottolineato: bool, barrato: bool) -> Run {
    Run { sottolineato, barrato, ..r(testo, false, false) }
}

#[test]
fn sottolineato_e_barrato_round_trip() {
    let blocchi = vec![Blocco::paragrafo(vec![
        decorato("sotto", true, false),
        decorato(" barrato", false, true),
        decorato(" entrambi", true, true),
        r(" niente", false, false),
    ])];
    let dati = in_memoria(&blocchi);
    let xml = parte_zip(&dati, "word/document.xml");
    assert!(xml.contains(r#"<w:rPr><w:u w:val="single"/></w:rPr>"#), "{xml}");
    assert!(xml.contains("<w:rPr><w:strike/></w:rPr>"), "{xml}");
    assert_eq!(leggi_byte(dati).blocchi, blocchi);
}

#[test]
fn sottolineato_e_barrato_da_altri_programmi() {
    let stili = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="character" w:styleId="Link"><w:name w:val="Hyperlink"/><w:rPr><w:u w:val="single"/></w:rPr></w:style></w:styles>"#;
    let document = format!(
        r#"<w:document {W}><w:body><w:p><w:r><w:rPr><w:u w:val="double"/></w:rPr><w:t>doppia</w:t></w:r><w:r><w:rPr><w:u w:val="none"/></w:rPr><w:t xml:space="preserve"> no</w:t></w:r><w:r><w:rPr><w:dstrike/></w:rPr><w:t xml:space="preserve"> due</w:t></w:r><w:r><w:rPr><w:strike w:val="0"/></w:rPr><w:t xml:space="preserve"> spento</w:t></w:r><w:r><w:rPr><w:rStyle w:val="Link"/></w:rPr><w:t xml:space="preserve"> stile</w:t></w:r></w:p></w:body></w:document>"#
    );
    let d = leggi_byte(pacchetto(&document, Some(stili), &[]));
    assert_eq!(
        d.blocchi,
        vec![Blocco::paragrafo(vec![
            decorato("doppia", true, false),
            decorato(" no", false, false),
            decorato(" due", false, true),
            decorato(" spento", false, false),
            decorato(" stile", true, false),
        ])]
    );
    assert!(d.non_supportati.is_empty(), "{:?}", d.non_supportati);
}

#[test]
fn modello_conserva_e_toglie_le_decorazioni() {
    let stili = r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="character" w:styleId="Link"><w:name w:val="Hyperlink"/><w:rPr><w:u w:val="single"/></w:rPr></w:style></w:styles>"#;
    let document = format!(
        r#"<w:document {W}><w:body><w:p><w:r><w:rPr><w:u w:val="double" w:color="FF0000"/></w:rPr><w:t>uno</w:t></w:r><w:r><w:rPr><w:dstrike/></w:rPr><w:t xml:space="preserve"> due</w:t></w:r><w:r><w:rPr><w:rStyle w:val="Link"/></w:rPr><w:t xml:space="preserve"> tre</w:t></w:r><w:r><w:t xml:space="preserve"> quattro</w:t></w:r></w:p></w:body></w:document>"#
    );
    let originale = pacchetto(&document, Some(stili), &[]);

    // senza modifiche resta tutto com'era: doppia, colorata, barrato doppio
    let xml = parte_zip(&risalva(originale.clone(), |_| {}), "word/document.xml");
    assert!(xml.contains(r#"<w:u w:val="double" w:color="FF0000"/>"#), "{xml}");
    assert!(xml.contains("<w:dstrike/>"), "{xml}");
    assert!(!xml.contains("<w:strike"), "{xml}");
    assert_eq!(xml.matches(r#"w:val="single""#).count(), 0, "lo sottolinea già lo stile: {xml}");

    // tolte e messe coi comandi
    let dati = risalva(originale, |b| {
        b[0].run[0].sottolineato = false;
        b[0].run[1].barrato = false;
        b[0].run[2].sottolineato = false;
        b[0].run[3].sottolineato = true;
        b[0].run[3].barrato = true;
    });
    let xml = parte_zip(&dati, "word/document.xml");
    assert!(!xml.contains("double"), "{xml}");
    assert!(!xml.contains("dstrike"), "{xml}");
    assert!(xml.contains(r#"<w:rStyle w:val="Link"/><w:u w:val="none"/>"#), "{xml}");
    assert!(xml.contains(r#"<w:rPr><w:strike/><w:u w:val="single"/></w:rPr>"#), "{xml}");
    let letti = &leggi_byte(dati).blocchi[0].run;
    assert_eq!(letti.len(), 2, "{letti:?}");
    assert_eq!(letti[0], decorato("uno due tre", false, false));
    assert_eq!(letti[1], decorato(" quattro", true, true));
}

#[test]
fn modello_il_testo_senza_formato_resta_senza() {
    let document = format!(
        r#"<w:document {W}><w:body><w:p><w:pPr><w:spacing w:after="400"/></w:pPr><w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t>Rosso</w:t></w:r><w:r><w:t xml:space="preserve"> e semplice</w:t></w:r></w:p><w:p><w:r><w:t>Paragrafo semplice</w:t></w:r></w:p></w:body></w:document>"#
    );
    let originale = pacchetto(&document, None, &[]);
    let xml = parte_zip(&risalva(originale.clone(), |_| {}), "word/document.xml");
    assert_eq!(xml.matches("FF0000").count(), 1, "{xml}");
    assert_eq!(xml.matches(r#"w:after="400""#).count(), 1, "{xml}");
    assert!(xml.contains(r#"<w:p><w:r><w:t xml:space="preserve">Paragrafo semplice"#), "{xml}");

    // il testo nuovo invece eredita: dal paragrafo prima, qui quello semplice
    let xml = parte_zip(&risalva(originale, |b| b.push(p("Nuovo"))), "word/document.xml");
    assert!(xml.contains(r#"<w:p><w:r><w:t xml:space="preserve">Nuovo"#), "{xml}");
}
