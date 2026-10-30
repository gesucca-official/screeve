// Finestra di domanda dentro l'app, nello stile di Screeve (i messaggi nativi di
// Windows non permettono di scegliere il pulsante predefinito né le etichette).

export interface Scelta<T> { etichetta: string; valore: T }

const box = document.querySelector('#dialog') as HTMLElement;
const titolo = box.querySelector('#dialog-title') as HTMLElement;
const testo = box.querySelector('#dialog-text') as HTMLElement;
const pulsanti = box.querySelector('#dialog-buttons') as HTMLElement;

let chiudi: ((i: number) => void) | null = null;

export const dialogoAperto = () => chiudi !== null;

// La prima scelta è quella predefinita (Invio); Esc restituisce `annulla`.
export function chiedi<T>(domanda: string, dettaglio: string, scelte: Scelta<T>[], annulla: T): Promise<T> {
  chiudi?.(-1);
  titolo.textContent = domanda;
  testo.textContent = dettaglio;
  testo.hidden = !dettaglio;
  pulsanti.replaceChildren(...scelte.map((s, i) => {
    const b = document.createElement('button');
    b.textContent = s.etichetta;
    b.className = i === 0 ? 'primary' : '';
    b.addEventListener('click', () => chiudi?.(i));
    return b;
  }));
  box.classList.add('open');
  (pulsanti.firstElementChild as HTMLElement).focus();

  return new Promise(resolve => {
    chiudi = i => {
      chiudi = null;
      box.classList.remove('open');
      resolve(i < 0 ? annulla : scelte[i].valore);
    };
  });
}

// Mentre la domanda è aperta le scorciatoie dell'app tacciono: tastiera solo per i pulsanti.
box.addEventListener('keydown', e => {
  e.stopPropagation();
  const tutti = [...pulsanti.children] as HTMLElement[];
  const i = tutti.indexOf(document.activeElement as HTMLElement);
  if (e.key === 'Escape') { e.preventDefault(); chiudi?.(-1); }
  else if (e.key === 'Tab' || e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
    e.preventDefault();
    const passo = e.key === 'ArrowLeft' || (e.key === 'Tab' && e.shiftKey) ? -1 : 1;
    tutti[(i + passo + tutti.length) % tutti.length].focus();
  }
});
