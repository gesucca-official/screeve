# Screeve

Editor di testo per la scrittura creativa in italiano: senza distrazioni, piacevole da guardare, leggero e portable.

- Carta crema e inchiostro marrone, colonna di testo a misura d'occhio
- Titolo, sottotitolo, grassetto e corsivo: niente di più
- Caratteri italiani a portata di mano: È « » ‹ ›
- Correttore ortografico e sinonimi in italiano, usabili da tastiera
- File `.docx` compatibili con Word e Google Documenti

## Stato

In sviluppo. Già pronti l'editor (fase 2: titolo, sottotitolo, grassetto e corsivo, caratteri italiani e correzioni automatiche, zoom, impostazioni con cinque font inclusi e guida, F1) e i file `.docx` (fase 3: Nuovo, Apri, Salva e Salva con nome, apertura dei documenti di Word, Google Documenti e LibreOffice con avviso su ciò che Screeve non gestisce). Mancano ancora correttore e sinonimi.

Il mockup interattivo è in [`mockup/index.html`](mockup/index.html) (si apre direttamente nel browser), il piano di implementazione in [`PIANO.md`](PIANO.md).

Stack: Tauri 2, TypeScript, ProseMirror, Rust.

## Sviluppo

Requisiti (Windows): [Node.js](https://nodejs.org) 20+, [Rust](https://rustup.rs) e i Microsoft C++ Build Tools. WebView2 è già presente in Windows 10 e 11.

```bash
npm install
npm run app:dev      # app in modalità sviluppo, con ricarica automatica
npm run app:build    # eseguibile portable in src-tauri/target/release/screeve.exe
```

`npm run dev` avvia solo l'interfaccia nel browser, su http://localhost:1420 (lì aprire e salvare non funzionano: servono le API di Tauri).

I test del docx (lettura, scrittura, round-trip) sono in Rust:

```bash
cd src-tauri
cargo test
cargo test -- --ignored genera_esempio   # rigenera docs/esempio.docx, da aprire a mano in Word e Google Documenti
```

I font vengono dai pacchetti Fontsource; dopo averli aggiornati, `node scripts/genera-font.mjs` rigenera `src/styles/fonts.css`.
