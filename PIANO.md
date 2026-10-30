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
| Font | File WOFF2/TTF **inclusi** nell'eseguibile (licenza OFL) | Niente rete, aspetto identico ovunque. |

Alternative scartate: Electron (troppo pesante), Wails/Go (valido, ma l'ecosistema Rust ha già spellbook e Tauri è più maturo), Neutralino (API native troppo limitate).

### Portabilità
- Build `tauri build` → un singolo `screeve.exe`, nessuna installazione.
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

Uso gli stili **integrati** di Word (`Heading1/2`) e non stili personalizzati: così il riquadro di spostamento di Word e la struttura di Google Documenti mostrano i capitoli, e l'indice automatico funziona. *(Alternativa: `Title`/`Subtitle`, ma Word li pensa come uno solo per documento.)*

### Scrittura
File minimo e valido: `[Content_Types].xml`, `_rels/.rels`, `word/document.xml`, `word/styles.xml`, `word/settings.xml`, `docProps/core.xml` e `app.xml`. Lingua `it-IT` impostata negli stili, così anche il correttore di Word lavora in italiano. Salvataggio **atomico** (scrivo su file temporaneo, poi rinomino) per non perdere mai un documento.

### Lettura (onestà sui limiti)
"100% compatibile" vale in modo pieno per **i file creati da Screeve**. Un docx arbitrario può contenere tabelle, immagini, note, commenti, revisioni: Screeve non li mostra. Proposta:
- importo testo, paragrafi, titoli (riconosco anche `Title`/`Subtitle`/`Heading3+` e i nomi localizzati tipo "Titolo 1"), grassetto e corsivo;
- se il file contiene altro, avviso chiaro all'apertura: *"Questo documento contiene elementi che Screeve non gestisce (2 immagini, 1 tabella). Salvando andrebbero persi."* e propongo **Salva una copia** come azione predefinita, per non distruggere mai l'originale.

### Verifica della compatibilità
- Test automatici di round-trip (docx → modello → docx) su un corpus di file prodotti da Word, Google Documenti e LibreOffice.
- Validazione XML contro gli schemi OOXML.
- Checklist manuale a ogni release: apertura in Word desktop, Word online, Google Documenti, LibreOffice.

## Correttore ortografico

- Il dizionario si carica in un thread Rust all'avvio (~qualche centinaio di ms), l'editor è subito utilizzabile.
- Il frontend invia, con debounce, solo i paragrafi modificati; Rust restituisce gli intervalli errati; ProseMirror li disegna come **decorazioni** (sottolineatura ondulata terracotta), quindi i refusi non entrano mai nel documento né nel docx.
- Suggerimenti calcolati solo su richiesta (apertura popover o Alt+Invio).
- Tokenizzazione italiana: elisioni (`dell'acqua` → `dell'` + `acqua`), apostrofo dritto e tipografico `'` `’` equivalenti, numeri e URL ignorati.
- **Dizionario personale** per nomi propri e parole inventate (fondamentale nella narrativa), più "Ignora" per la sessione.
- Regole extra italiane oltre al dizionario: `qual'è` → `qual è`, `po` → `po'`, `un'altro` → `un altro`, `perchè/poichè/affinchè` → accento acuto.

## Interfaccia

- **Barra in alto, su due righe:** sopra Titolo · Sottotitolo · Testo | G · C | Sinonimi; sotto i caratteri È « » ‹ › «…». A sinistra il nome del file, a destra il contatore dei refusi e due icone: **Impostazioni** e **Guida**.
- **Barra in basso:** parole, caratteri, tempo di lettura e lo zoom `− 100% +`.
- Mentre si scrive, entrambe le barre si ritirano quasi del tutto.

## Impostazioni

Pannello laterale che si apre dall'icona con i cursori (o **Ctrl+,**). Il testo si sposta di lato per restare visibile, così ogni modifica si vede subito.
- **Font**, divisi in due gruppi:
  - *Raffinati:* Literata (predefinito), Source Serif 4, Newsreader, Lora, EB Garamond, Crimson Pro.
  - *Corposi:* Merriweather, Vollkorn, Bitter, Alegreya.
  
  Ogni font ha il suo corpo ideale, applicato quando lo si sceglie; poi si può regolare.
- **Dimensione** di base (predefinita 19 px) e **larghezza della colonna** (predefinita ~65 caratteri per riga), con l'indicazione dei caratteri per riga effettivi.
- **Correzioni automatiche** attivabili e disattivabili.
- **Ripristina predefiniti.**

Le impostazioni si salvano in `screeve.json` (vedi Portabilità). I 10 font, ognuno in 4 stili, aggiungono qualche MB all'eseguibile.

## Guida

Icona "i" in alto a destra (o **F1**): finestra sovrapposta, scorrevole, chiusa da Esc o col clic fuori. Contiene: scrivere senza distrazioni, struttura del testo, caratteri italiani e correzioni automatiche, refusi, sinonimi, dimensione del testo, impostazioni, file docx, e la tabella completa delle scorciatoie. Il testo della guida è già scritto nel mockup e lo riusiamo così com'è.

## Sinonimi

- Si seleziona una parola (o basta avere il cursore sopra) e si preme **Maiusc+F7** (come in Word) o il pulsante **Sinonimi**.
- Popover con i sinonimi raggruppati per significato; si sceglie con frecce + Invio o col clic. La sostituzione conserva maiuscola iniziale, corsivo e grassetto ed è annullabile con Ctrl+Z.
- Fonte: **thesaurus italiano di LibreOffice** (formato MyThes `th_it_IT_v2.idx/.dat`), letto in Rust con ricerca nell'indice, senza caricarlo tutto in memoria. Licenza da verificare prima del rilascio.
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
| Impostazioni | Ctrl+, |
| Guida | F1 |
| Chiudi pannelli e suggerimenti | Esc |
| Salva / Apri / Nuovo | Ctrl+S / Ctrl+O / Ctrl+N |

Correzioni automatiche mentre scrivi (disattivabili): `<<` → «, `>>` → », `E'` → È.

## Aspetto

- Carta crema `#F4EDDE`, inchiostro marrone `#3A2A1F`, accento cuoio `#8C5A3C`, refusi terracotta `#B4533A`.
- Colonna di testo a larghezza fissa, ~65 caratteri per riga (l'intervallo ottimale per la lettura), centrata; i margini crescono con la finestra, il testo no. ✅ Confermato.
- Interlinea 1,65; spaziatura tra paragrafi.
- Font predefinito Literata a 19 px; gli altri si scelgono dalle Impostazioni (tutti con licenza OFL).

## Fasi

1. **Scheletro** — progetto Tauri 2 + Vite + TS, finestra, tema, font incluso, build portable funzionante.
2. **Editor** — schema ProseMirror ristretto, barra su due righe con titolo/sottotitolo/testo, G/C, caratteri speciali, avvolgi in « », correzioni automatiche, conteggio parole, ingrandimento del testo, UI che si ritira, pannello Impostazioni, Guida.
3. **DOCX** — Nuovo/Apri/Salva/Salva con nome, scrittura e poi lettura, avviso sui contenuti non supportati, test di round-trip e prova manuale in Word e Google Documenti.
4. **Correttore** — spellbook + it_IT, decorazioni, popover, F7/Alt+Invio, dizionario personale, regole italiane.
5. **Sinonimi** — thesaurus MyThes, popover, lemma dalla forma flessa (MVP), poi sinonimi già flessi.
6. **Rifinitura** — salvataggio automatico e recupero dopo crash, impostazioni portable, file recenti, icona, firma dell'eseguibile.

Idee per dopo (fuori dal minimo): modalità macchina da scrivere (riga corrente sempre al centro), focus sul paragrafo corrente, tema notte, lineetta di dialogo `—` con `--`, obiettivo di parole giornaliero, esportazione PDF/EPUB.

## Decisioni aperte

1. **Font nel docx**: scrivo nel file il font dell'editor (chi lo apre senza averlo installato vedrà un sostituto) o un font standard (es. Garamond / Times New Roman) e uso il nostro solo a schermo?
2. **Titolo/Sottotitolo** come `Heading1/2` (proposto) o `Title/Subtitle`?
3. **Paragrafi**: spaziatura tra paragrafi (come ora) o rientro della prima riga, all'uso dei libri italiani?
4. Supporto anche a **`.odt`** o ad altri formati più avanti?
