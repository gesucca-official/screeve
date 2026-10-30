import { invoke, isTauri } from '@tauri-apps/api/core';

// Preferenze della postazione, in screeve.json accanto all'eseguibile (o in
// %APPDATA%\Screeve, vedi src-tauri/src/dati.rs). Il localStorage della WebView ne
// tiene una copia: all'avvio serve subito, prima che Rust risponda.

export interface Font {
  name: string;    // come appare nelle impostazioni
  family: string;  // la famiglia CSS (vedi styles/fonts.css)
  size: number;    // il corpo ideale, applicato quando lo si sceglie
  note: string;
}

// Tutti a tratto pieno: niente filetti sottili che sullo schermo si perdono.
// Due classici e tre voci diverse, per cambiare aria mentre si scrive.
export const FONTS: Font[] = [
  { name: 'Literata',              family: 'Literata Variable',          size: 19,   note: 'disegnato per i libri digitali' },
  { name: 'Libre Baskerville',     family: 'Libre Baskerville Variable', size: 17.5, note: 'ampio ed elegante, molto arioso' },
  { name: 'Zilla Slab',            family: 'Zilla Slab',                 size: 20,   note: 'graziato squadrato, deciso e simpatico' },
  { name: 'Atkinson Hyperlegible', family: 'Atkinson Hyperlegible',      size: 18.5, note: 'senza grazie, nato per la massima leggibilità' },
  { name: 'Courier Prime',         family: 'Courier Prime',              size: 18,   note: 'la macchina da scrivere, ripensata per lo schermo' },
];

export interface Settings {
  font: string;     // nome del font in FONTS
  size: number;     // px, al 100%
  measure: number;  // larghezza della colonna in em (33em ≈ 65 caratteri per riga)
  auto: boolean;    // correzioni automatiche
  zoom: number;     // 1 = 100%
}

export const DEFAULTS: Readonly<Settings> = { font: 'Literata', size: 19, measure: 33, auto: true, zoom: 1 };

const KEY = 'screeve-settings';

// Valori mancanti o del tipo sbagliato tornano ai predefiniti.
function valid(text: string | null): Settings {
  const s: Settings = { ...DEFAULTS };
  try {
    const saved = JSON.parse(text || '{}');
    for (const k of Object.keys(DEFAULTS) as (keyof Settings)[]) {
      if (typeof saved[k] === typeof DEFAULTS[k]) (s as any)[k] = saved[k];
    }
  } catch { /* impostazioni illeggibili: restano i predefiniti */ }
  if (!FONTS.some(f => f.name === s.font)) s.font = DEFAULTS.font;
  return s;
}

// Dalla copia nel localStorage: immediata.
export function loadSettings(): Settings {
  try { return valid(localStorage.getItem(KEY)); } catch { return { ...DEFAULTS }; }
}

// Da screeve.json; null se il file non c'è ancora (o nel browser).
export async function loadSettingsFile(): Promise<Settings | null> {
  if (!isTauri()) return null;
  const text = await invoke<string | null>('leggi_impostazioni').catch(() => null);
  return text === null ? null : valid(text);
}

let timer: number | undefined;

// I cursori mandano un valore a ogni pixel: il file si scrive quando si fermano.
export function saveSettings(s: Settings): void {
  try { localStorage.setItem(KEY, JSON.stringify(s)); } catch { /* non è grave */ }
  if (!isTauri()) return;
  clearTimeout(timer);
  const text = JSON.stringify(s, null, 2) + '\n';
  timer = setTimeout(() => invoke('salva_impostazioni', { testo: text }).catch(() => {}), 400);
}

export const fontByName = (name: string): Font => FONTS.find(f => f.name === name) ?? FONTS[0];
