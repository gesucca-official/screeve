import { invoke, isTauri } from '@tauri-apps/api/core';
import { TextSelection } from 'prosemirror-state';
import type { EditorView } from 'prosemirror-view';
import { replaceWord, wordAt } from './editor';

// Sinonimi della parola sotto il cursore (Maiusc+F7 o il pulsante): il riquadro
// prende il focus, si sceglie con le frecce e Invio, Esc torna al testo. Il
// thesaurus ha solo forme base, quindi per "antiche" i sinonimi sono di «antico».

interface Significato { categoria: string; sinonimi: string[] }
interface Voce { lemma: string; significati: Significato[] }

export interface SinonimiOptions {
  view: EditorView;
  flash: (testo: string) => void;
  chiudiAltri: () => void;  // il riquadro dei refusi
}

export function gestioneSinonimi({ view, flash, chiudiAltri }: SinonimiOptions) {
  const app = isTauri();
  let parola: { from: number; to: number; text: string } | null = null;
  const pop = document.querySelector('#pop-syn') as HTMLElement;

  async function apri(): Promise<void> {
    if (!app) { flash('I sinonimi funzionano nell\'app, non nel browser'); return; }
    const w = wordAt(view.state);
    if (!w) { flash('Metti il cursore su una parola per cercarne i sinonimi'); return; }
    let voci: Voce[] | null;
    try {
      voci = await invoke<Voce[] | null>('sinonimi', { parola: w.text });
    } catch (e) {
      flash(String(e));
      return;
    }
    if (voci === null) { flash('Il thesaurus si sta caricando'); return; }
    // nel frattempo il testo potrebbe essere cambiato
    if (view.state.doc.textBetween(w.from, w.to) !== w.text) return;
    chiudiAltri();
    parola = w;
    view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, w.from, w.to)));
    disegna(w.text, voci);
  }

  function disegna(testo: string, voci: Voce[]): void {
    const el = (tag: string, cls: string, t = '') => {
      const e = document.createElement(tag);
      e.className = cls;
      e.textContent = t;
      return e;
    };
    const head = el('div', 'head', 'Sinonimi di ');
    head.append(el('b', '', testo));
    const maiuscola = /^\p{Lu}/u.test(testo);
    const cap = (s: string) => maiuscola ? s.charAt(0).toUpperCase() + s.slice(1) : s;
    const gruppi = voci.flatMap(v => {
      // il lemma si dice quando non è la parola scritta: chi scrive adatta la desinenza
      const da = v.lemma.toLowerCase() !== testo.toLowerCase() ? [el('div', 'lemma', `da «${v.lemma}»`)] : [];
      let prima = '';
      return [...da, ...v.significati.map(s => {
        const g = el('div', 'group');
        // "sostantivo femminile" ripetuto a ogni significato è solo rumore
        const label = s.categoria === prima ? [] : [el('div', 'label', s.categoria)];
        prima = s.categoria;
        const chips = el('div', 'chips');
        chips.append(...s.sinonimi.map(x => {
          const b = el('button', 'chip', cap(x));
          b.dataset.syn = cap(x);
          return b;
        }));
        g.append(...label, chips);
        return g;
      })];
    });
    const vuoto = voci.length ? [] : [el('div', 'empty', 'Nessun sinonimo trovato.')];
    const hint = el('div', 'hint');
    hint.innerHTML = '<kbd>←</kbd> <kbd>→</kbd> scegli &nbsp;·&nbsp; <kbd>Invio</kbd> sostituisci &nbsp;·&nbsp; <kbd>Esc</kbd> chiudi';
    pop.replaceChildren(head, ...gruppi, ...vuoto, hint);
    pop.classList.add('open');
    posiziona();
    (pop.querySelector('.chip') as HTMLElement | null)?.focus({ preventScroll: true });
  }

  // sotto la parola, oppure sopra se in fondo alla finestra non c'è spazio
  function posiziona(): void {
    if (!parola) return;
    const a = view.coordsAtPos(parola.from), b = view.coordsAtPos(parola.to);
    pop.style.left = Math.max(8, Math.min(a.left, innerWidth - pop.offsetWidth - 8)) + 'px';
    const sotto = b.bottom + 8;
    const top = sotto + pop.offsetHeight > innerHeight - 8 ? a.top - pop.offsetHeight - 8 : sotto;
    pop.style.top = Math.max(8, top) + 'px';
  }

  function chiudi(): void {
    if (!pop.classList.contains('open')) return;
    pop.classList.remove('open');
    parola = null;
    view.focus();
  }

  function sostituisci(nuova: string): void {
    const w = parola;
    chiudi();
    if (!w || view.state.doc.textBetween(w.from, w.to) !== w.text) return;
    view.dispatch(replaceWord(view.state, w.from, w.to, nuova).scrollIntoView());
    flash(`Sostituito con «${nuova}»`);
  }

  pop.addEventListener('mousedown', e => { if (!(e.target as HTMLElement).closest('.chip')) e.preventDefault(); });
  pop.addEventListener('click', e => {
    const b = (e.target as HTMLElement).closest<HTMLElement>('.chip');
    if (b) sostituisci(b.dataset.syn!);
  });
  pop.addEventListener('keydown', e => {
    // mentre il riquadro è aperto, la tastiera serve solo a lui
    e.stopPropagation();
    if (e.key === 'Escape') { e.preventDefault(); chiudi(); return; }
    const chips = [...pop.querySelectorAll<HTMLElement>('.chip')];
    const i = chips.indexOf(document.activeElement as HTMLElement);
    const passo = ({ ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 } as Record<string, number>)[e.key];
    if (passo && chips.length) {
      e.preventDefault();
      const prossimo = chips[(i + passo + chips.length) % chips.length];
      prossimo.focus({ preventScroll: true });
      prossimo.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter' && i >= 0) {
      e.preventDefault();
      sostituisci(chips[i].dataset.syn!);
    } else if (e.key === 'Tab') {
      e.preventDefault();
    }
  });
  document.addEventListener('mousedown', e => {
    if (pop.classList.contains('open') && !pop.contains(e.target as HTMLElement)) chiudi();
  });
  addEventListener('resize', posiziona);

  return { apri, chiudi, aperto: () => pop.classList.contains('open') };
}
