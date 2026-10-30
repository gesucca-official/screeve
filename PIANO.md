# Screeve — piano di implementazione

Editor per la scrittura creativa in italiano: senza distrazioni, piacevole da guardare, leggero e portable.
Mockup interattivo: [mockup/index.html](mockup/index.html).

## Principi

1. **Meno è meglio.** Ogni funzione deve guadagnarsi il posto. Niente menu annidati, niente pannelli.
2. **La pagina è tutto.** L'interfaccia si ritira mentre scrivi e torna quando muovi il mouse.
3. **L'italiano prima di tutto.** Caporali, È, accenti corretti (perché/perchè), elisioni (dell'acqua).
4. **Il file è dell'utente.** Un `.docx` scritto da Screeve si apre identico in Word e Google Documenti.

## Stack proposto

| Livello | Scelta | Perché |
|---|---|---|
| Shell desktop | **Tauri 2** (Rust) | Un solo `.exe` di ~8 MB, usa WebView2 già presente in Windows 10/11. Electron peserebbe ~150 MB. |
| Interfaccia | **TypeScript + Vite**, nessun framework | L'UI è una barra e una pagina: un framework sarebbe solo peso. |
| Editor | **ProseMirror** | Modello documento rigoroso: lo schema ammette *solo* ciò che il docx sa rappresentare (titolo, sottotitolo, paragrafo, grassetto, corsivo), quindi nessuna formattazione "orfana" può nascere. Undo/redo, input rules e decorazioni (per i refusi) già pronti. `contenteditable` puro (come nel mockup) non è affidabile per un editor vero. |
| Correttore | **spellbook** (crate Rust, compatibile Hunspell) + dizionario **it_IT** di LibreOffice | Veloce e nativo. In JS (nspell) il dizionario italiano è notoriamente lento e pesante in memoria. |
| DOCX | Scrittura e lettura **in Rust** con `zip` + `quick-xml`, OOXML generato a mano | Lo schema è minuscolo: generare noi l'XML ci dà il controllo totale su ciò che finisce nel file, quindi la compatibilità è garantita e testabile. |
| Font | File WOFF2 **inclusi** nell'eseguibile (licenza OFL), dai pacchetti **Fontsource** | Niente rete, aspetto identico ovunque. Solo i sottoinsiemi latin e latin-ext (`scripts/genera-font.mjs` → `src/styles/fonts.css`). |

Alternative scartate: Electron (troppo pesante), Wails/Go (valido, ma l'ecosistema Rust ha già spellbook e Tauri è più maturo), Neutralino (API native troppo limitate).

### Portabilità
- Build `tauri build` → `screeve.exe` più la cartella `dizionari/` (dizionario e thesaurus, che per licenza restano file separati, vedi Licenza). Si distribuisce come zip, nessuna installazione.
- Impostazioni e dizionario personale in `screeve.json` / `personale.dic` **accanto all'eseguibile** se la cartella è scrivibile (chiavetta USB), altrimenti in `%APPDATA%\Screeve`.
- Requisito: WebView2 (preinstallato su Windows 11 e sui Windows 10 aggiornati). Più avanti: build macOS/Linux con lo stesso codice.

## Modello del documento e mappatura DOCX

| Screeve | ProseMirror | DOCX (`w:pStyle`) | Google Documenti |
|---|---|---|---|
| Titolo | `heading` livello 1 | `Heading1` | Titolo 1 |
| Sottotitolo | `heading` livello 2 | `Heading2` | Titolo 2 |
| Testo | `paragraph` | `Normal` | Testo normale |
| Grassetto | mark `strong` | `<w:b/>` | Grassetto |
| Corsivo | mark `em` | `<w:i/>` | Corsivo |

Uso gli stili **integrati** di Word (`Heading1/2`) e non stili personalizzati: così il riquadro di spostamento di Word e la struttura di Google Documenti mostrano i capitoli, e l'indice automatico funziona. ✅ Deciso.

### Scrittura
File minimo e valido: `[Content_Types].xml`, `_rels/.rels`, `word/document.xml` con le sue relazioni (`word/_rels/document.xml.rels`), `word/styles.xml`, `word/settings.xml`, `docProps/core.xml` e `app.xml`. Lingua `it-IT` impostata negli stili, così anche il correttore di Word lavora in italiano. Pagina A4 con i margini predefiniti di Word in italiano, `compatibilityMode` 15 (niente "Modalità compatibilità"). Codice in `src-tauri/src/docx/`.

Decisioni ✅:
- **Font nel file: Georgia**, presente in Word (Windows e Mac) e in Google Documenti, a prescindere dal font scelto a schermo. Il font dell'editor è solo una preferenza di visualizzazione.
- **Paragrafi neutri:** né spaziatura prima/dopo né rientro della prima riga, interlinea singola, testo a 12 pt. L'impaginazione (es. per la stampa) si fa poi in Word con le impostazioni di paragrafo; la spaziatura che si vede nell'editor è solo a schermo.
- **Titolo** (`heading 1`): 22 pt grassetto, 24 pt prima e 6 dopo. **Sottotitolo** (`heading 2`): 14 pt corsivo, 6 pt prima e 18 dopo. Entrambi restano attaccati al paragrafo che segue.
- Niente `.odt` per ora. Salvataggio **atomico** (scrivo su file temporaneo, poi rinomino) per non perdere mai un documento.

### Lettura (onestà sui limiti)
"100% compatibile" vale in modo pieno per **i file creati da Screeve**. Un docx arbitrario può contenere tabelle, immagini, note, commenti, revisioni: Screeve non li mostra. Proposta:
- importo testo, paragrafi, titoli, grassetto e corsivo;
- lo stile si risolve tramite `styles.xml`, perché gli id possono essere localizzati (`Titolo1` in Word italiano): conta il **nome** dello stile (`heading 1`, `Title`, anche "Titolo 1"), poi `outlineLvl`, poi lo stile di base. `Title` e `Heading1` → Titolo; `Subtitle` e `Heading2` o inferiori → Sottotitolo;
- grassetto e corsivo valgono anche quando arrivano dallo stile del carattere (es. "Enfasi grassetto") o da uno stile di paragrafo (es. "Citazione" in corsivo), e si spengono con `w:val="0"`/`"false"`; stili di paragrafo e di carattere si annullano a vicenda come in Word. Nei titoli lo stile del paragrafo non conta: il grassetto di un titolo è il suo aspetto, non una marcatura;
- tabulazioni: quelle a inizio paragrafo (rientri fatti a mano) si tolgono, le altre restano. A capo manuale: nel testo divide il paragrafo, in un titolo diventa uno spazio. Campi (numeri di pagina, indici…): resta il risultato visibile. Revisioni: si legge il testo come se fossero accettate. Il testo delle tabelle diventa paragrafi;
- se il file contiene altro (tabelle, immagini, caselle di testo, formule, note, commenti, revisioni, elenchi, intestazioni e piè di pagina, interruzioni di pagina, campi, collegamenti, sottolineature, simboli) lo conto e all'apertura mostro un avviso chiaro: *"Questo documento contiene elementi che Screeve non gestisce. 2 immagini e 1 tabella. Salvando andrebbero persi…"* con **Salva una copia** come scelta predefinita, per non distruggere mai l'originale. Anche chiudendo l'avviso con Esc il documento diventa una copia non ancora salvata.

### Verifica della compatibilità
- `cargo test` (in `src-tauri`): round-trip modello → docx → modello, XML ben formato, salvataggio atomico, e lettura di XML costruiti come quelli di Word in italiano, Google Documenti e LibreOffice, con il conteggio degli elementi non supportati.
- `cargo test -- --ignored genera_esempio` rigenera [`docs/esempio.docx`](docs/esempio.docx) (titoli, sottotitoli, grassetto, corsivo, caporali, accenti) per la prova manuale.
- Più avanti: un corpus di file veri prodotti da Word, Google Documenti e LibreOffice, e validazione XML contro gli schemi OOXML.
- Checklist manuale a ogni release: apertura in Word desktop, Word online, Google Documenti, LibreOffice.

## Correttore ortografico

- Il dizionario si carica in un thread Rust all'avvio (~qualche centinaio di ms), l'editor è subito utilizzabile.
- Il frontend invia, con debounce, solo i paragrafi modificati; Rust restituisce gli intervalli errati; ProseMirror li disegna come **decorazioni** (sottolineatura ondulata terracotta), quindi i refusi non entrano mai nel documento né nel docx.
- Suggerimenti calcolati solo su richiesta (apertura popover o Alt+Invio).
- Tokenizzazione italiana: elisioni (`dell'acqua` → `dell'` + `acqua`), apostrofo dritto e tipografico `'` `’` equivalenti, numeri e URL ignorati.
- **Dizionario personale** per nomi propri e parole inventate (fondamentale nella narrativa), più "Ignora" per la sessione.
- Regole extra italiane oltre al dizionario: `qual'è` → `qual è`, `po` → `po'`, `un'altro` → `un altro`, `perchè/poichè/affinchè` → accento acuto.

## Interfaccia

- **Barra in alto:** Titolo · Sottotitolo · Testo | G · C | Sinonimi. A sinistra il nome del file con lo stato (*salvato*, *modificato*, *non salvato*, ripetuto nel titolo della finestra con un pallino •), a destra il contatore dei refusi e due icone: **Impostazioni** e **Guida**.
- **File:** Nuovo, Apri, Salva, Salva con nome, dal piccolo menu che si apre cliccando sul nome del file o con le scorciatoie. Prima di Nuovo, Apri o della chiusura della finestra con modifiche non salvate: *Salva · Non salvare · Annulla*. Si apre anche trascinando un .docx sulla finestra o sull'eseguibile, con "Apri con" o col doppio clic su un file associato (il percorso arriva come argomento da riga di comando). Domande e avvisi sono finestre dell'app, nello stile della pagina.
- **Barra in basso:** parole, caratteri, tempo di lettura e lo zoom `− 100% +`.
- **Caratteri, colonna verticale sul lato destro, nella parte bassa:** È « » «…». Quando il pannello Impostazioni è aperto restano coperti.
- Mentre si scrive, barre e caratteri si ritirano quasi del tutto.

## Impostazioni

Pannello laterale che si apre dall'icona con i cursori (o **Ctrl+,**). Il testo si sposta di lato per restare visibile, così ogni modifica si vede subito.
- **Font**, cinque, tutti a tratto pieno (niente filetti sottili): due classici, Literata (predefinito) e Libre Baskerville, e tre voci diverse per cambiare aria mentre si scrive: Zilla Slab (graziato squadrato), Atkinson Hyperlegible (senza grazie) e Courier Prime (macchina da scrivere; essendo monospaziato, a parità di colonna sta su ~55 caratteri per riga). **Ctrl+Maiusc+F** passa al font successivo senza aprire le impostazioni. Ogni font ha il suo corpo ideale, applicato quando lo si sceglie; poi si può regolare.
- **Dimensione** di base (predefinita 19 px) e **larghezza della colonna** (predefinita ~65 caratteri per riga), con l'indicazione dei caratteri per riga effettivi.
- **Correzioni automatiche** attivabili e disattivabili.
- **Ripristina predefiniti.**

Le impostazioni si salvano in `screeve.json` (vedi Portabilità), con una copia nel `localStorage` della WebView per averle subito all'avvio. I 5 font, ognuno in 4 stili (latin e latin-ext, woff2), aggiungono circa 1 MB all'eseguibile.

## Guida

Icona "i" in alto a destra (o **F1**): finestra sovrapposta, scorrevole, chiusa da Esc o col clic fuori. Contiene: scrivere senza distrazioni, struttura del testo, caratteri italiani e correzioni automatiche, refusi, sinonimi, dimensione del testo, impostazioni, file docx, e la tabella completa delle scorciatoie. Il testo della guida è già scritto nel mockup e lo riusiamo così com'è.

## Sinonimi

- Si seleziona una parola (o basta avere il cursore sopra) e si preme **Maiusc+F7** (come in Word) o il pulsante **Sinonimi**.
- Popover con i sinonimi raggruppati per significato; si sceglie con frecce + Invio o col clic. La sostituzione conserva maiuscola iniziale, corsivo e grassetto ed è annullabile con Ctrl+Z.
- Fonte: **thesaurus italiano di LibreOffice** (formato MyThes `th_it_IT_v2.dat`, 2 MB): si tiene in memoria con un indice costruito all'avvio (~25 ms), le voci si leggono solo quando servono. GPL 3 come il dizionario: anche lui resta un file separato in `dizionari/`.
- **Il punto difficile è la flessione.** Il thesaurus contiene forme base (*antico*, *brillare*), ma nel testo si scrive *antiche*, *brilla*. Piano a due passi:
  1. **MVP:** dalla forma flessa si risale al lemma con le regole del dizionario Hunspell (le stesse del correttore) e si mostrano i sinonimi del lemma, dicendo chiaramente "da «antico»". Chi scrive adatta la desinenza.
  2. **Poi:** i sinonimi vengono proposti già nella forma giusta (*vecchie*, *risplende*): regole per sostantivi e aggettivi regolari, e per i verbi la stessa persona e lo stesso tempo, usando le forme generate dal dizionario. Il mockup mostra già questo comportamento.

## Ingrandimento del testo

- **Ctrl++ / Ctrl+−** (anche Ctrl+rotellina), **Ctrl+0** per tornare al 100%; in più i pulsanti − 100% + discreti nella barra di stato.
- Scalini da 70% a 200%. Testo e colonna crescono insieme, quindi i caratteri per riga restano gli stessi finché la finestra lo consente; su schermi stretti la colonna si adatta alla finestra.
- È una preferenza della postazione (salvata nelle impostazioni), non del documento: il docx non cambia.
- In Tauri lo zoom nativo della WebView resta disattivato, così le scorciatoie le gestiamo noi.

## Scorciatoie

| Azione | Tasti |
|---|---|
| Grassetto / Corsivo | Ctrl+B / Ctrl+I |
| Titolo / Sottotitolo / Testo | Ctrl+Alt+1 / Ctrl+Alt+2 / Ctrl+Alt+0 (come Word e Google Documenti) |
| Racchiudi selezione in « » | Ctrl+Shift+' |
| Prossimo refuso | F7 |
| Correggi refuso (sotto il cursore o appena scritto) | Alt+Invio |
| Scegli il 2°/3° suggerimento | Alt+2 / Alt+3 |
| Sinonimi della parola | Maiusc+F7 |
| Ingrandisci / Rimpicciolisci / 100% | Ctrl++ / Ctrl+− / Ctrl+0 |
| Font successivo | Ctrl+Maiusc+F |
| Impostazioni | Ctrl+, |
| Guida | F1 |
| Chiudi pannelli e suggerimenti | Esc |
| Salva / Apri / Nuovo | Ctrl+S / Ctrl+O / Ctrl+N |
| Salva con nome | Ctrl+Maiusc+S |

Correzioni automatiche mentre scrivi (disattivabili): `<<` → «, `>>` → », `E'` → È.

## Aspetto

- Carta crema `#F4EDDE`, inchiostro marrone `#3A2A1F`, accento cuoio `#8C5A3C`, refusi terracotta `#B4533A`.
- Colonna di testo a larghezza fissa, ~65 caratteri per riga (l'intervallo ottimale per la lettura), centrata; i margini crescono con la finestra, il testo no. ✅ Confermato.
- Interlinea 1,65; spaziatura tra paragrafi.
- Font predefinito Literata a 19 px; gli altri si scelgono dalle Impostazioni (tutti con licenza OFL).

## Fasi

1. ✅ **Scheletro** — progetto Tauri 2 + Vite + TS, finestra, tema, font incluso, build portable funzionante.
2. ✅ **Editor** — schema ProseMirror ristretto, barra con titolo/sottotitolo/testo e G/C, caratteri speciali in colonna sul lato destro, avvolgi in « », correzioni automatiche, conteggio parole, ingrandimento del testo, UI che si ritira, pannello Impostazioni, Guida.
3. ✅ **DOCX** — Nuovo/Apri/Salva/Salva con nome, scrittura e poi lettura, avviso sui contenuti non supportati, test di round-trip. Provato a mano in Word e Google Documenti. Menu File cliccabile sul nome del file.
4. ✅ **Correttore** — spellbook + it_IT, decorazioni, popover, F7/Alt+Invio/Alt+2-5, Ignora, dizionario personale, regole italiane. Codice in `src-tauri/src/correttore/` e `src/correttore.ts`, test con `cargo test correttore`. Il dizionario personale sta già accanto all'eseguibile o in `%APPDATA%\Screeve` (`src-tauri/src/dati.rs`).
5. ✅ **Sinonimi (MVP)** — thesaurus MyThes, popover con frecce/Invio/Esc, lemma dalla forma flessa con le regole SFX del dizionario al contrario più i singolari (case → casa, antiche → antico, brillavano → brillare, dirtelo → dire). Codice in `src-tauri/src/sinonimi/` e `src/sinonimi.ts`. Limiti: i passati remoti irregolari (corse → correre) non si trovano. Resta per dopo: sinonimi già flessi.
6. **Rifinitura** — ✅ copia di sicurezza e recupero dopo un crash (`src-tauri/src/recupero.rs`: copia in `recupero/` nella cartella dei dati 2 s dopo l'ultima modifica, un lucchetto per sessione così due finestre non si confondono, il .docx non si tocca mai); ✅ impostazioni in `screeve.json` (il localStorage resta come copia per l'avvio); ✅ icona (le caporali « », da `app-icon.svg`); ✅ zip di distribuzione con le licenze (`npm run app:pacchetto`). Restano: file recenti, firma dell'eseguibile.

Idee per dopo (fuori dal minimo): modalità macchina da scrivere (riga corrente sempre al centro), focus sul paragrafo corrente, tema notte, lineetta di dialogo `—` con `--`, obiettivo di parole giornaliero, esportazione PDF/EPUB.

## Licenza ✅

- Il codice di Screeve è **tutti i diritti riservati** ([LICENSE](LICENSE)): il repository è pubblico ma non concede permessi. In `package.json` è `UNLICENSED`.
- Dizionario e thesaurus italiani di LibreOffice sono **GPL 3**. Per non estendere la GPL a Screeve restano **file di dati separati e non modificati** in `dizionari/` accanto all'eseguibile (non incorporati con `include_bytes!`), con il testo della licenza e il link ai sorgenti. Le regole italiane extra (qual'è, po'…) sono codice nostro, non modifiche al dizionario.
- spellbook è MPL-2.0: si usa senza modificarne i file. Font OFL, librerie MIT/Apache.
- Elenco completo in [TERZE-PARTI.md](TERZE-PARTI.md); i testi delle licenze entrano nello zip in fase 6.
- Non è una consulenza legale: prima di vendere o distribuire su larga scala conviene una verifica.
