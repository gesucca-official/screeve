import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Node } from 'prosemirror-model';
import { TextSelection } from 'prosemirror-state';
import { Decoration, DecorationSet, type EditorView } from 'prosemirror-view';
import { elencoRefusi, refuso, refusiKey, replaceWord, type Intervallo, type RefusiMeta } from './editor';

// Correttore ortografico: invia a Rust (src-tauri/src/correttore) i paragrafi
// cambiati, disegna i refusi come decorazioni e gestisce F7, Alt+Invio, Alt+2/3 e
// il riquadro dei suggerimenti. Il riquadro non prende il focus: si continua a
// scrivere, e si sceglie col mouse o con Alt+Invio / Alt+numero.

interface Refuso { inizio: number; fine: number; parola: string }  // posizioni nel testo del paragrafo
interface Aperto extends Intervallo { parola: string; suggerimenti: string[] }

const ATTESA = 350;  // ms dopo l'ultima modifica

export interface CorrettoreOptions {
  view: EditorView;
  flash: (testo: string) => void;
}

export function gestioneCorrettore({ view, flash }: CorrettoreOptions) {
  const app = isTauri();
  // i nodi di ProseMirror sono immutabili: un paragrafo non cambiato è lo stesso
  // oggetto, quindi si ricontrollano solo quelli nuovi
  const controllati = new WeakMap<Node, Refuso[]>();
  const ignorate = new Set<string>();  // "Ignora", per questa sessione
  let stato: 'attesa' | 'pronto' | 'assente' = 'attesa';
  let timer: number | undefined;
  let inCorso: Promise<void> | null = null;
  let ancora = false;
  let aperto: Aperto | null = null;

  const pulsante = document.querySelector('#spell') as HTMLElement;
  const etichetta = document.querySelector('#spell-label') as HTMLElement;
  const pop = document.querySelector('#pop') as HTMLElement;

  const dispatch = (meta: RefusiMeta) =>
    view.dispatch(view.state.tr.setMeta(refusiKey, meta).setMeta('addToHistory', false));

  /* ---------- controllo ---------- */
  function programma(ms = ATTESA): void {
    clearTimeout(timer);
    timer = setTimeout(controlla, ms);
  }

  function controlla(): Promise<void> {
    if (!app || stato === 'assente') return Promise.resolve();
    if (inCorso) { ancora = true; return inCorso; }
    inCorso = (async () => {
      const nuovi: Node[] = [];
      view.state.doc.forEach(n => { if (!controllati.has(n)) nuovi.push(n); });
      if (nuovi.length) {
        const r = await invoke<Refuso[][] | null>('controlla', { testi: nuovi.map(n => n.textContent) });
        if (r === null) { aggiornaEtichetta(); return; }  // arriverà "correttore-pronto"
        stato = 'pronto';
        nuovi.forEach((n, i) => controllati.set(n, r[i]));
      }
      ridisegna();
    })().catch(e => {
      stato = 'assente';
      pulsante.title = String(e);
      aggiornaEtichetta();
      flash(String(e));
    }).finally(() => {
      inCorso = null;
      if (ancora) { ancora = false; programma(0); }
    });
    return inCorso;
  }

  // Tutte le decorazioni da capo; i paragrafi non ancora controllati tengono le loro.
  function ridisegna(): void {
    const { doc } = view.state;
    const prima = refusiKey.getState(view.state)!.set;
    const decorazioni: Decoration[] = [];
    doc.forEach((n, pos) => {
      const r = controllati.get(n);
      if (!r) { decorazioni.push(...prima.find(pos, pos + n.nodeSize)); return; }
      for (const x of r) {
        if (!ignorate.has(norm(x.parola))) decorazioni.push(refuso(pos + 1 + x.inizio, pos + 1 + x.fine, x.parola));
      }
    });
    dispatch({ set: DecorationSet.create(doc, decorazioni) });
    aggiornaEtichetta();
  }

  function aggiornaEtichetta(): void {
    if (!app) return;
    const n = elencoRefusi(view.state).length;
    etichetta.textContent = stato === 'attesa' ? 'dizionario…' : stato === 'assente' ? 'correttore spento'
      : n === 0 ? 'nessun refuso' : n === 1 ? '1 refuso' : `${n} refusi`;
    pulsante.classList.toggle('typos', stato === 'pronto' && n > 0);
    pulsante.classList.toggle('clean', stato === 'pronto' && n === 0);
  }

  /** Da chiamare a ogni transazione dell'editor. */
  function cambiato(docCambiato: boolean): void {
    if (!aperto) { if (docCambiato) programma(); return; }
    const { from, to } = view.state.selection;
    if (docCambiato || from < aperto.from || to > aperto.to) chiudi();
    if (docCambiato) programma();
  }

  /* ---------- suggerimenti ---------- */
  function pronto(): boolean {
    if (!app) flash('Il correttore funziona nell\'app, non nel browser');
    else if (stato === 'attesa') flash('Il dizionario si sta caricando');
    else if (stato === 'assente') flash(pulsante.title || 'Correttore non disponibile');
    return app && stato === 'pronto';
  }

  async function chiediSuggerimenti(parola: string): Promise<string[]> {
    try { return await invoke<string[]>('suggerisci', { parola }); } catch { return []; }
  }

  async function apri(r: Intervallo): Promise<void> {
    const parola = view.state.doc.textBetween(r.from, r.to);
    const suggerimenti = await chiediSuggerimenti(parola);
    // nel frattempo il testo potrebbe essere cambiato
    if (view.state.doc.textBetween(r.from, r.to) !== parola) return;
    aperto = { ...r, parola, suggerimenti };
    dispatch({ attuale: r });
    disegnaPop(aperto);
  }

  function disegnaPop({ parola, suggerimenti }: Aperto): void {
    const el = (tag: string, cls: string, testo = '') => {
      const e = document.createElement(tag);
      e.className = cls;
      e.textContent = testo;
      return e;
    };
    const head = el('div', 'head');
    head.append(el('s', '', parola), suggerimenti.length ? '  forse cercavi' : '');
    const voci = suggerimenti.map((s, i) => {
      const b = el('button', 'sugg');
      b.dataset.i = String(i);
      b.append(el('span', '', s), el('kbd', '', i === 0 ? 'Alt+Invio' : `Alt+${i + 1}`));
      return b;
    });
    const vuoto = suggerimenti.length ? [] : [el('div', 'empty', 'Nessun suggerimento')];
    const foot = el('div', 'foot');
    const ignora = el('button', '', 'Ignora');
    ignora.dataset.act = 'ignora';
    const aggiungi = el('button', '', 'Aggiungi al dizionario');
    aggiungi.dataset.act = 'aggiungi';
    foot.append(ignora, aggiungi);
    pop.replaceChildren(head, ...voci, ...vuoto, foot);
    pop.classList.add('open');
    posiziona();
  }

  // sotto la parola, oppure sopra se in fondo alla finestra non c'è spazio
  function posiziona(): void {
    if (!aperto) return;
    const a = view.coordsAtPos(aperto.from), b = view.coordsAtPos(aperto.to);
    pop.style.left = Math.max(8, Math.min(a.left, innerWidth - pop.offsetWidth - 8)) + 'px';
    const sotto = b.bottom + 8;
    pop.style.top = (sotto + pop.offsetHeight > innerHeight - 8 ? a.top - pop.offsetHeight - 8 : sotto) + 'px';
  }

  function chiudi(): void {
    if (!aperto && !pop.classList.contains('open')) return;
    aperto = null;
    pop.classList.remove('open');
    if (refusiKey.getState(view.state)!.attuale) dispatch({ attuale: null });
  }

  pop.addEventListener('mousedown', e => e.preventDefault());  // il focus resta nel testo
  pop.addEventListener('click', e => {
    const b = (e.target as HTMLElement).closest('button');
    if (!b || !aperto) return;
    const { parola } = aperto;
    if (b.dataset.i) correggi(aperto, aperto.suggerimenti[+b.dataset.i]);
    else if (b.dataset.act === 'ignora') ignora(parola);
    else if (b.dataset.act === 'aggiungi') aggiungi(parola);
  });
  addEventListener('resize', posiziona);
  addEventListener('scroll', posiziona, { passive: true });

  /* ---------- azioni ---------- */
  function correggi(r: Intervallo, nuova: string): void {
    chiudi();
    const tr = replaceWord(view.state, r.from, r.to, nuova);
    tr.setMeta(refusiKey, { togli: { from: r.from, to: tr.selection.from } } satisfies RefusiMeta);
    view.dispatch(tr);
    view.focus();
    flash(`Corretto: ${view.state.doc.textBetween(r.from, view.state.selection.from)}`);
  }

  function ignora(parola: string): void {
    ignorate.add(norm(parola));
    chiudi();
    ridisegna();
    view.focus();
  }

  async function aggiungi(parola: string): Promise<void> {
    try {
      await invoke('aggiungi_parola', { parola });
    } catch (e) {
      flash(String(e));
      return;
    }
    ignorate.add(norm(parola));
    chiudi();
    ridisegna();
    view.focus();
    flash(`«${parola}» aggiunto al dizionario`);
  }

  // F7: il refuso dopo il cursore (o dopo quello aperto), ricominciando dall'inizio.
  async function successivo(): Promise<void> {
    if (!pronto()) return;
    await controllaOra();
    const tutti = elencoRefusi(view.state);
    if (!tutti.length) { chiudi(); flash('Nessun refuso, ottimo lavoro'); return; }
    const da = aperto ? aperto.from : view.state.selection.from;
    const r = tutti.find(x => x.from > da) ?? tutti[0];
    view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, r.from, r.to)).scrollIntoView());
    view.focus();
    await apri(r);
  }

  // Alt+Invio, Alt+2, Alt+3: il refuso aperto, quello sotto il cursore o l'ultimo
  // scritto nello stesso paragrafo, senza staccare le mani dalla tastiera.
  async function veloce(n: number): Promise<void> {
    if (!pronto()) return;
    let r: Intervallo | undefined = aperto ?? undefined;
    if (!r) {
      await controllaOra();
      const { $from, from } = view.state.selection;
      const tutti = elencoRefusi(view.state).filter(x => x.from >= $from.start() && x.to <= $from.end());
      r = tutti.find(x => x.from <= from && from <= x.to) ?? tutti.filter(x => x.to <= from).pop();
    }
    if (!r) { flash('Nessun refuso qui'); return; }
    const parola = view.state.doc.textBetween(r.from, r.to);
    const s = aperto?.suggerimenti ?? await chiediSuggerimenti(parola);
    if (!s[n - 1]) { flash(s.length ? `Solo ${s.length} suggeriment${s.length === 1 ? 'o' : 'i'}` : `Nessun suggerimento per «${parola}»`); return; }
    if (view.state.doc.textBetween(r.from, r.to) === parola) correggi(r, s[n - 1]);
  }

  async function controllaOra(): Promise<void> {
    clearTimeout(timer);
    await controlla();
  }

  /* ---------- avvio ---------- */
  function avvia(): void {
    aggiornaEtichetta();
    if (!app) return;
    listen('correttore-pronto', () => programma(0));
    programma(0);
  }

  return {
    avvia,
    cambiato,
    /** documento sostituito (Nuovo, Apri) */
    ricomincia: () => { chiudi(); programma(0); },
    clic: (r: Intervallo | null) => { if (r) apri(r); else chiudi(); },
    successivo,
    veloce,
    chiudi,
    aperto: () => aperto !== null,
  };
}

const norm = (s: string) => s.replace(/’/g, "'");
