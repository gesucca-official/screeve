# Componenti di terze parti

Il codice di Screeve è "tutti i diritti riservati" (vedi [LICENSE](LICENSE)). Questi componenti hanno le loro licenze, che restano valide.

## Inclusi nell'eseguibile

| Componente | Licenza | Uso |
|---|---|---|
| Literata, Libre Baskerville, Zilla Slab, Atkinson Hyperlegible, Courier Prime (pacchetti Fontsource) | SIL Open Font License 1.1 | Font dell'editor |
| ProseMirror (`prosemirror-*`) | MIT | Editor |
| Tauri e `@tauri-apps/*`, plugin dialog | MIT o Apache-2.0 | Finestra e API native |
| serde, serde_json | MIT o Apache-2.0 | Dati tra interfaccia e Rust |
| zip | MIT | Lettura e scrittura del .docx |
| quick-xml | MIT | XML del .docx |
| spellbook (dalla fase 4) | MPL-2.0 | Correttore ortografico, usato senza modifiche |

## Distribuiti accanto all'eseguibile, nella cartella `dizionari/`

| Componente | Licenza | Uso |
|---|---|---|
| Dizionario italiano `it_IT.aff` / `it_IT.dic` (Italian Writing Aids, LibreOffice) | GNU GPL 3 | Correttore ortografico |
| Thesaurus italiano `th_it_IT_v2` (LibreOffice) | GNU GPL 3 | Sinonimi |

Dizionario e thesaurus sono file di dati **separati e non modificati**, distribuiti insieme a Screeve ma non incorporati nell'eseguibile: accanto a loro vanno il testo della GPL 3, i README originali e il link ai sorgenti, <https://github.com/LibreOffice/dictionaries/tree/master/it_IT>.

Il testo completo di queste licenze va incluso nella distribuzione (fase 6). Strumenti usati solo per costruire l'app, come Vite e TypeScript, non finiscono nell'eseguibile.
