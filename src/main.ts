import 'prosemirror-view/style/prosemirror.css';
import './styles/fonts.css';
import './styles/theme.css';
import './styles/app.css';

import type { EditorView } from 'prosemirror-view';
import type { Command } from 'prosemirror-state';
import {
  createEditor, currentBlock, insertChar, markActive, replaceDoc, setBlock, toggleEm, toggleStrong, wrapGuillemets,
  type Block,
} from './editor';
import { schema } from './editor/schema';
import { gestioneCorrettore } from './correttore';
import { gestioneFile } from './file';
import { gestioneSinonimi } from './sinonimi';
import { adattaScorciatoie, modificatore } from './piattaforma';
import { DEFAULTS, FONTS, fontByName, loadSettings, loadSettingsFile, saveSettings, type Font, type Settings } from './settings';

const $ = <T extends HTMLElement = HTMLElement>(sel: string) => document.querySelector(sel) as T;
const root = document.documentElement.style;

/* ---------- editor ---------- */
let settings = loadSettings();
let lastDoc: unknown = null;

const view = createEditor($('#editor'), {
  enabled: () => settings.auto,
  onApply: flash,
  onChange(v) {
    refreshToolbar(v);
    const changed = v.state.doc !== lastDoc;
    if (changed) { lastDoc = v.state.doc; updateCountSoon(); file.aggiorna(); }
    spell.cambiato(changed);
  },
  onTypoClick: r => spell.clic(r),
});

/* ---------- correttore ---------- */
const spell = gestioneCorrettore({ view, flash });
const syn = gestioneSinonimi({ view, flash, chiudiAltri: () => spell.chiudi() });

/* ---------- file ---------- */
const file = gestioneFile({
  view,
  flash,
  sostituisci(doc) {
    replaceDoc(view, doc);
    lastDoc = view.state.doc;
    refreshToolbar(view);
    updateCount();
    spell.ricomincia();
    scrollTo(0, 0);
  },
});

/* ---------- menu File, sul nome del file ---------- */
const fileMenu = $('#file-menu');
const fileItems = () => [...fileMenu.querySelectorAll<HTMLElement>('button')];

function toggleFileMenu(force?: boolean): void {
  const wasOpen = fileMenu.classList.contains('open');
  const open = fileMenu.classList.toggle('open', force);
  $('#file-name').classList.toggle('on', open);
  $('#file-name').setAttribute('aria-expanded', String(open));
  if (open) fileItems()[0].focus();
  else if (wasOpen) view.focus();
}
$('#file-name').addEventListener('click', () => toggleFileMenu());
fileMenu.addEventListener('click', e => {
  const b = (e.target as HTMLElement).closest<HTMLElement>('[data-file]');
  if (!b) return;
  toggleFileMenu(false);
  file[b.dataset.file as 'nuovo' | 'apri' | 'salva' | 'salvaConNome']();
});
fileMenu.addEventListener('keydown', e => {
  if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
  e.preventDefault();
  const items = fileItems();
  const i = items.indexOf(document.activeElement as HTMLElement);
  items[(i + (e.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length].focus();
});
document.addEventListener('mousedown', e => {
  if (!(e.target as HTMLElement).closest('.file-wrap')) toggleFileMenu(false);
});

const run = (cmd: Command) => { cmd(view.state, view.dispatch, view); view.focus(); };

/* ---------- barra in alto e caratteri ---------- */
function refreshToolbar(v: EditorView): void {
  const block = currentBlock(v.state);
  document.querySelectorAll<HTMLElement>('[data-block]').forEach(b => b.classList.toggle('on', b.dataset.block === block));
  document.querySelectorAll<HTMLElement>('[data-mark]').forEach(b =>
    b.classList.toggle('on', markActive(v.state, schema.marks[b.dataset.mark!])));
}

function onToolMousedown(e: MouseEvent): void {
  const b = (e.target as HTMLElement).closest('button');
  if (!b) return;
  e.preventDefault(); // non rubare il focus all'editor
  if (b.dataset.block) run(setBlock(b.dataset.block as Block));
  else if (b.dataset.mark) run(b.dataset.mark === 'strong' ? toggleStrong : toggleEm);
  else if (b.dataset.char) run(insertChar(b.dataset.char));
  else if (b.id === 'b-wrap') run(wrapGuillemets);
  else if (b.id === 'b-syn') synonyms();
  else if (b.id === 'spell') nextTypo();
}
$('.tools').addEventListener('mousedown', onToolMousedown);
$('#chars').addEventListener('mousedown', onToolMousedown);
$('#spell').addEventListener('mousedown', onToolMousedown);

const nextTypo = () => spell.successivo();
const synonyms = () => syn.apri();

/* ---------- impostazioni ---------- */
function applySettings(): void {
  const font = fontByName(settings.font);
  root.setProperty('--font', `"${font.family}"`);
  root.setProperty('--size', settings.size + 'px');
  root.setProperty('--measure', settings.measure + 'em');
  root.setProperty('--zoom', String(settings.zoom));
  document.querySelectorAll<HTMLElement>('#fonts button').forEach(b => b.classList.toggle('on', b.dataset.font === font.name));
  $<HTMLInputElement>('#size').value = String(settings.size);
  $('#size-val').textContent = settings.size.toLocaleString('it-IT') + ' px';
  $<HTMLInputElement>('#measure').value = String(settings.measure);
  $<HTMLInputElement>('#auto').checked = settings.auto;
  $('#zoom-val').textContent = Math.round(settings.zoom * 100) + '%';
  updateCplSoon();
}

function setSetting(patch: Partial<Settings>): void {
  Object.assign(settings, patch);
  saveSettings(settings);
  applySettings();
}

$('#fonts').innerHTML = FONTS.map(f =>
  `<button data-font="${f.name}"><span style="font-family:'${f.family}'">${f.name}</span><small>${f.note}</small></button>`).join('');

// ogni font ha il suo corpo ideale
const setFont = (f: Font) => setSetting({ font: f.name, size: f.size });

function nextFont(): void {
  const f = FONTS[(FONTS.indexOf(fontByName(settings.font)) + 1) % FONTS.length];
  setFont(f);
  flash(f.name);
}

$('#fonts').addEventListener('click', e => {
  const b = (e.target as HTMLElement).closest('button');
  if (b) setFont(fontByName(b.dataset.font!));
});
$('#size').addEventListener('input', e => setSetting({ size: +(e.target as HTMLInputElement).value }));
$('#measure').addEventListener('input', e => setSetting({ measure: +(e.target as HTMLInputElement).value }));
$('#auto').addEventListener('change', e => setSetting({ auto: (e.target as HTMLInputElement).checked }));
$('#reset').addEventListener('click', () => {
  setSetting({ ...DEFAULTS, zoom: settings.zoom }); // lo zoom ha già Ctrl+0
  flash('Impostazioni predefinite ripristinate');
});

function toggleSettings(force?: boolean): void {
  const open = $('#settings').classList.toggle('open', force);
  $('#b-settings').classList.toggle('on', open);
  if (!open) view.focus();
  updateCplSoon();
}
$('#b-settings').addEventListener('click', () => toggleSettings());
$('#settings-close').addEventListener('click', () => toggleSettings(false));

/* ---------- guida ---------- */
const guide = $('#guide');
guide.tabIndex = -1; // riceve il focus, così frecce e PagGiù la fanno scorrere

function toggleGuide(force?: boolean): void {
  const open = guide.classList.toggle('open', force);
  if (open) { guide.scrollTop = 0; guide.focus(); }
  else view.focus();
}
$('#b-guide').addEventListener('click', () => toggleGuide());
$('#guide-close').addEventListener('click', () => toggleGuide(false));
guide.addEventListener('click', e => { if (e.target === guide) toggleGuide(false); });

/* ---------- ingrandimento del testo ---------- */
const ZOOMS = [0.7, 0.8, 0.9, 1, 1.1, 1.25, 1.4, 1.6, 1.8, 2];

function zoomBy(step: number): void {
  const i = ZOOMS.indexOf(settings.zoom);
  const next = step === 0 ? 1 : ZOOMS[Math.max(0, Math.min(ZOOMS.length - 1, (i < 0 ? 3 : i) + step))];
  setSetting({ zoom: next });
}

$('.zoom').addEventListener('mousedown', e => {
  const b = (e.target as HTMLElement).closest('button');
  if (!b) return;
  e.preventDefault();
  zoomBy(b.id === 'zoom-in' ? 1 : b.id === 'zoom-out' ? -1 : 0);
});

addEventListener('wheel', e => {
  // su macOS il pizzico del trackpad arriva come rotellina con Ctrl
  if (!e.ctrlKey && !modificatore(e)) return;
  e.preventDefault();
  zoomBy(e.deltaY < 0 ? 1 : -1);
}, { passive: false });

/* ---------- tastiera ---------- */
// Le scorciatoie di scrittura (Ctrl+B, Ctrl+Alt+1…) sono nel keymap dell'editor; qui quelle dell'app.
document.addEventListener('keydown', e => {
  const ctrl = modificatore(e) && !e.altKey;
  if (e.key === 'F1') { e.preventDefault(); toggleGuide(); return; }
  if (e.key === 'Escape') {
    if (spell.aperto()) spell.chiudi();
    else if (fileMenu.classList.contains('open')) toggleFileMenu(false);
    else if (guide.classList.contains('open')) toggleGuide(false);
    else if ($('#settings').classList.contains('open')) toggleSettings(false);
    return;
  }
  if (ctrl && e.key === ',') { e.preventDefault(); toggleSettings(); return; }
  const key = e.key.toLowerCase();
  if (ctrl && ['n', 'o', 's'].includes(key)) {
    e.preventDefault();
    if (e.repeat) return;
    toggleFileMenu(false);
    if (key === 'n') file.nuovo();
    else if (key === 'o') file.apri();
    else if (e.shiftKey) file.salvaConNome();
    else file.salva();
    return;
  }
  // Alt+Invio, Alt+2, Alt+3…: correggi il refuso col primo, secondo, terzo suggerimento
  const alt = e.altKey && !e.ctrlKey && !e.metaKey;
  if (alt && (e.key === 'Enter' || /^Digit[2-5]$/.test(e.code))) {
    e.preventDefault();
    spell.veloce(e.key === 'Enter' ? 1 : +e.code.slice(5));
    return;
  }
  if (ctrl && e.shiftKey && e.code === 'KeyF') { e.preventDefault(); nextFont(); return; }
  if (e.key === 'F7') {
    e.preventDefault();
    if (e.shiftKey) synonyms(); else nextTypo();
    return;
  }
  if (ctrl && ['+', '=', '-', '0'].includes(e.key)) {
    e.preventDefault();
    zoomBy(e.key === '0' ? 0 : e.key === '-' ? -1 : 1);
    return;
  }
  // Nell'app ricaricare, stampare o cercare con la barra del browser non ha senso
  // (e ricaricare farebbe perdere il testo). In sviluppo restano attivi.
  if (!import.meta.env.DEV && (e.key === 'F5' || (ctrl && ['r', 'p', 'f', 'g', 'h', 'j', 'u'].includes(e.key.toLowerCase())))) {
    e.preventDefault();
  }
});

/* ---------- interfaccia che si ritira mentre scrivi ---------- */
view.dom.addEventListener('keydown', e => {
  if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) document.body.classList.add('typing');
});
let lx: number | undefined, ly = 0;
document.addEventListener('mousemove', e => {
  if (lx !== undefined && Math.hypot(e.clientX - lx, e.clientY - ly) > 4) document.body.classList.remove('typing');
  lx = e.clientX; ly = e.clientY;
});

// Fuori dal testo il menu contestuale del browser (Indietro, Aggiorna…) non ha senso.
document.addEventListener('contextmenu', e => {
  if (!view.dom.contains(e.target as Node)) e.preventDefault();
});

/* ---------- conteggi e messaggi ---------- */
const fmt = (n: number) => n.toLocaleString('it-IT');
// "dell'acqua" conta come una parola, come in Word
const WORD = /[\p{L}\p{N}]+(?:['’][\p{L}\p{N}]+)*/gu;

function updateCount(): void {
  const { doc } = view.state;
  const text = doc.textBetween(0, doc.content.size, '\n');
  const words = text.match(WORD)?.length ?? 0;
  const chars = text.replace(/\n/g, '').length;
  const reading = words ? ` · ${Math.max(1, Math.round(words / 230))} min di lettura` : '';
  $('#count').textContent = `${fmt(words)} parole · ${fmt(chars)} caratteri${reading}`;
}
let countTimer: number | undefined;
function updateCountSoon(): void {
  clearTimeout(countTimer);
  countTimer = window.setTimeout(updateCount, 300);
}

let msgTimer: number | undefined;
function flash(text: string): void {
  $('#msg').textContent = text;
  clearTimeout(msgTimer);
  msgTimer = window.setTimeout(() => $('#msg').textContent = '', 2200);
}

/* ---------- caratteri per riga effettivi ---------- */
// Una riga di prosa italiana, misurata nel font e nel corpo attuali: vale anche col documento vuoto.
const ruler = $('#ruler');
ruler.textContent = 'Ogni sera, prima che il sole toccasse il mare, Elia saliva i centotrentadue gradini del faro contandoli a voce bassa. Non lo faceva per abitudine, ma perché temeva che un giorno ne avrebbe trovato uno in più.';

function updateCpl(): void {
  const avg = ruler.getBoundingClientRect().width / ruler.textContent!.length;
  $('#cpl').textContent = avg ? `≈ ${Math.round(view.dom.clientWidth / avg)} caratteri per riga` : '';
}
let cplTimer: number | undefined;
function updateCplSoon(): void {
  clearTimeout(cplTimer);
  // aspetta il font e la transizione della colonna (.3s)
  cplTimer = window.setTimeout(() => {
    document.fonts.load(`16px "${fontByName(settings.font).family}"`, ruler.textContent!).then(updateCpl, updateCpl);
  }, 320);
}
addEventListener('resize', updateCplSoon);

adattaScorciatoie();
applySettings();
// screeve.json vale più della copia nel localStorage; se non c'è ancora, lo si crea
loadSettingsFile().then(fromFile => {
  if (!fromFile) saveSettings(settings);
  else if (JSON.stringify(fromFile) !== JSON.stringify(settings)) setSetting(fromFile);
});
updateCount();
refreshToolbar(view);
file.avvia();
spell.avvia();
view.focus();
