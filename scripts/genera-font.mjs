// Genera src/styles/fonts.css dai pacchetti Fontsource: solo sottoinsiemi latin e latin-ext, solo woff2.
// Uso: node scripts/genera-font.mjs (dopo aver aggiornato i pacchetti dei font).
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const sources = [
  ['@fontsource-variable/literata', ['opsz.css', 'opsz-italic.css']],
  ['@fontsource-variable/libre-baskerville', ['wght.css', 'wght-italic.css']],
  ['@fontsource/zilla-slab', ['400.css', '400-italic.css', '600.css', '700.css', '700-italic.css']],
  ['@fontsource/atkinson-hyperlegible', ['400.css', '400-italic.css', '700.css', '700-italic.css']],
  ['@fontsource/courier-prime', ['400.css', '400-italic.css', '700.css', '700-italic.css']],
];
let out = '/* Font inclusi (licenza OFL), dai pacchetti Fontsource.\n   Solo i sottoinsiemi latin e latin-ext e solo woff2: bastano per l\'italiano e pesano meno.\n   File generato da scripts/genera-font.mjs: non modificarlo a mano. */\n';
for (const [pkg, files] of sources) {
  for (const f of files) {
    const css = fs.readFileSync(`${root}/node_modules/${pkg}/${f}`, 'utf8');
    for (const block of css.split(/(?=\/\* )/)) {
      const m = block.match(/^\/\* ([^*]+) \*\//);
      if (!m || !/-latin(-ext)?-/.test(m[1])) continue;
      out += '\n' + block.trim()
        .replace(/url\(\.\/files\/([^)]+\.woff2)\)/, `url('${pkg}/files/$1')`)
        .replace(/, url\([^)]+\.woff\) format\('woff'\)/, '') + '\n';
    }
  }
}
fs.writeFileSync(`${root}/src/styles/fonts.css`, out);
