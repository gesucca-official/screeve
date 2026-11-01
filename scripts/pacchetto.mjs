// Prepara lo zip da distribuire (su macOS: Screeve.app, con i dizionari nelle risorse
// del bundle, e le licenze): screeve.exe, la cartella dizionari, LICENSE e
// TERZE-PARTI.md, e in licenze/ i testi delle licenze di tutto ciò che finisce
// nell'eseguibile (pacchetti npm dell'interfaccia e crate Rust per Windows o macOS).
// Uso: npm run app:pacchetto (fa prima la build).
import { execFileSync, execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const tauri = path.join(root, 'src-tauri');
const release = path.join(tauri, 'target', 'release');
const { version } = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const mac = process.platform === 'darwin';
const nome = `Screeve-${version}${mac ? '-macos' : ''}`;
const base = path.join(tauri, 'target', 'pacchetto');
const out = path.join(base, nome);

fs.rmSync(base, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
if (mac) {
  fs.cpSync(path.join(release, 'bundle', 'macos', 'Screeve.app'), path.join(out, 'Screeve.app'), { recursive: true, verbatimSymlinks: true });
} else {
  fs.copyFileSync(path.join(release, 'screeve.exe'), path.join(out, 'screeve.exe'));
  fs.cpSync(path.join(tauri, 'dizionari'), path.join(out, 'dizionari'), { recursive: true });
}
fs.copyFileSync(path.join(root, 'LICENSE'), path.join(out, 'LICENSE.txt'));
fs.copyFileSync(path.join(root, 'TERZE-PARTI.md'), path.join(out, 'TERZE-PARTI.md'));

const LICENZA = /^(licen[cs]e|copying|notice|copyright)/i;

// Copia i file di licenza di un pacchetto; se non ne ha, ne scrive il nome SPDX.
function licenze(da, a, spdx) {
  const file = fs.existsSync(da) ? fs.readdirSync(da).filter(f => LICENZA.test(f) && fs.statSync(path.join(da, f)).isFile()) : [];
  fs.mkdirSync(a, { recursive: true });
  for (const f of file) fs.copyFileSync(path.join(da, f), path.join(a, f));
  if (!file.length) fs.writeFileSync(path.join(a, 'LICENSE-SPDX.txt'), `${spdx ?? 'licenza non indicata'}\n`);
  return file.length > 0;
}

// npm: solo le dipendenze dell'interfaccia, non gli strumenti di build
const npm = execSync('npm ls --omit=dev --all --parseable', { cwd: root, encoding: 'utf8' })
  .split(/\r?\n/).filter(p => p && path.resolve(p) !== path.resolve(root));
let mancanti = [];
for (const dir of npm) {
  const pkg = JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
  if (!licenze(dir, path.join(out, 'licenze', 'npm', pkg.name.replace('/', '__')), pkg.license)) mancanti.push(pkg.name);
}

// Rust: i crate del grafo per la piattaforma (con qualche strumento di build in più: meglio
// abbondare che dimenticarne uno)
const triplo = mac ? (process.arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin') : 'x86_64-pc-windows-msvc';
const meta = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--filter-platform', triplo],
  { cwd: tauri, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
const usati = new Set(meta.resolve.nodes.map(n => n.id));
let crate = 0;
for (const p of meta.packages) {
  if (!usati.has(p.id) || p.name === 'screeve') continue;
  crate++;
  if (!licenze(path.dirname(p.manifest_path), path.join(out, 'licenze', 'rust', `${p.name}-${p.version}`), p.license)) mancanti.push(p.name);
}

const zip = path.join(base, `${nome}.zip`);
if (mac) {
  // ditto conserva link simbolici e attributi del bundle, che zip e tar rovinerebbero
  execFileSync('ditto', ['-c', '-k', '--keepParent', path.join(base, nome), zip]);
} else {
  // zip con il tar di Windows (quello di Git non sa fare gli zip)
  const tar = process.platform === 'win32' ? path.join(process.env.SystemRoot, 'System32', 'tar.exe') : 'tar';
  execFileSync(tar, ['-a', '-cf', zip, '-C', base, nome]);
}

console.log(`Pacchetto pronto: ${zip}`);
console.log(`Licenze: ${npm.length} pacchetti npm, ${crate} crate Rust` +
  (mancanti.length ? ` (senza file di licenza, scritto solo il nome SPDX: ${mancanti.join(', ')})` : ''));
