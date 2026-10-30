// Copia src-tauri/dizionari accanto all'eseguibile di rilascio. Dizionario e thesaurus
// sono GPL: restano file separati, non incorporati in screeve.exe (vedi TERZE-PARTI.md).
// Uso: lo lancia `npm run app:build` dopo la build.
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const da = root + 'src-tauri/dizionari';
const a = root + 'src-tauri/target/release/dizionari';
fs.rmSync(a, { recursive: true, force: true });
fs.cpSync(da, a, { recursive: true });
console.log(`Dizionari copiati in ${a}`);
