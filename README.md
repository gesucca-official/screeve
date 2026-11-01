# Screeve

Editor di testo per la scrittura creativa in italiano: senza distrazioni, piacevole da guardare, leggero e portable.

- Carta crema e inchiostro marrone, colonna di testo a misura d'occhio
- Titolo, sottotitolo, grassetto e corsivo: niente di più
- Caratteri italiani a portata di mano: È « »
- Correttore ortografico e sinonimi in italiano, usabili da tastiera
- File `.docx` compatibili con Word e Google Documenti

## Stato

In sviluppo. Già pronti l'editor (fase 2: titolo, sottotitolo, grassetto e corsivo, caratteri italiani e correzioni automatiche, zoom, impostazioni con cinque font inclusi e guida, F1), i file `.docx` (fase 3: Nuovo, Apri, Salva e Salva con nome, dal menu sul nome del file o da tastiera, apertura dei documenti di Word, Google Documenti e LibreOffice con avviso su ciò che Screeve non gestisce) il correttore ortografico (fase 4: refusi sottolineati, F7, Alt+Invio, dizionario personale) i sinonimi (fase 5: Maiusc+F7, anche dalle forme flesse) e buona parte della rifinitura (fase 6: copia di sicurezza e recupero dopo un crash, impostazioni portable in `screeve.json`, zip di distribuzione). Mancano file recenti e firma dell'eseguibile.

Il mockup interattivo è in [`mockup/index.html`](mockup/index.html) (si apre direttamente nel browser), il piano di implementazione in [`PIANO.md`](PIANO.md).

Stack: Tauri 2, TypeScript, ProseMirror, Rust.

Copyright © 2026 Simone Cipriani, tutti i diritti riservati ([LICENSE](LICENSE)). Componenti di terze parti e relative licenze: [TERZE-PARTI.md](TERZE-PARTI.md).

## Sviluppo

Requisiti (Windows e macOS): [Node.js](https://nodejs.org) 20+, [Rust](https://rustup.rs). Su Windows servono i Microsoft C++ Build Tools e WebView2 è già presente in Windows 10 e 11; su macOS (10.15 o successivo) bastano gli Xcode Command Line Tools (`xcode-select --install`).

```bash
npm install
npm run app:dev      # app in modalità sviluppo, con ricarica automatica
npm run app:build    # eseguibile portable in src-tauri/target/release/screeve.exe, con accanto la cartella dizionari/
npm run app:pacchetto  # build e poi lo zip da distribuire, con le licenze, in src-tauri/target/pacchetto/
```

Su macOS si costruisce `Screeve.app` (con i dizionari nelle risorse del bundle, firmato ad-hoc) e poi lo zip:

```bash
npm run app:build:mac      # src-tauri/target/release/bundle/macos/Screeve.app
npm run app:pacchetto:mac  # build e zip da distribuire in src-tauri/target/pacchetto/
```

Su macOS le scorciatoie usano ⌘ al posto di Ctrl, e le impostazioni stanno in `~/Library/Application Support/Screeve` (l'app non è portable: il bundle non si modifica). Scaricata da internet, l'app non è notarizzata: la prima volta si apre con clic destro → Apri.

`npm run dev` avvia solo l'interfaccia nel browser, su http://localhost:1420 (lì aprire e salvare non funzionano: servono le API di Tauri).

I test del docx (lettura, scrittura, round-trip), del correttore e dei sinonimi sono in Rust:

```bash
cd src-tauri
cargo test
cargo test -- --ignored genera_esempio   # rigenera docs/esempio.docx, da aprire a mano in Word e Google Documenti
```

I font vengono dai pacchetti Fontsource; dopo averli aggiornati, `node scripts/genera-font.mjs` rigenera `src/styles/fonts.css`.
