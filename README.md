# Screeve

Editor di testo per la scrittura creativa in italiano: senza distrazioni, piacevole da guardare, leggero e portable.

- Carta crema e inchiostro marrone, colonna di testo a misura d'occhio
- Titolo, sottotitolo, grassetto e corsivo: niente di più
- Caratteri italiani a portata di mano: È « » ‹ ›
- Correttore ortografico e sinonimi in italiano, usabili da tastiera
- File `.docx` compatibili con Word e Google Documenti

## Stato

In progettazione. Il mockup interattivo è in [`mockup/index.html`](mockup/index.html) (si apre direttamente nel browser), il piano di implementazione in [`PIANO.md`](PIANO.md).

Stack: Tauri 2, TypeScript, ProseMirror, Rust.

## Sviluppo

Requisiti (Windows): [Node.js](https://nodejs.org) 20+, [Rust](https://rustup.rs) e i Microsoft C++ Build Tools. WebView2 è già presente in Windows 10 e 11.

```bash
npm install
npm run app:dev      # app in modalità sviluppo, con ricarica automatica
npm run app:build    # eseguibile portable in src-tauri/target/release/screeve.exe
```

`npm run dev` avvia solo l'interfaccia nel browser, su http://localhost:1420.
