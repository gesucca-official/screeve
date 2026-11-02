import type { EditorView } from 'prosemirror-view';

// Modalità macchina da scrivere: la riga corrente resta al centro della finestra
// mentre si scrive o si muove il cursore coi tasti. Col mouse invece la pagina sta
// ferma: né un clic, né una selezione, né lo scorrimento a mano la fanno saltare.
// I margini in cima e in fondo sono in app.css (body.macchina).

const ATTESA_COLONNA = 320;  // ms: la transizione della colonna (.3s) cambia le righe

export function gestioneMacchina(view: EditorView, attiva: () => boolean) {
  let testa = view.state.selection.head;
  let selezione = false;  // c'era una selezione: chiuderla è una mossa anche se il cursore sta lì
  let colMouse = false;  // l'ultimo gesto è stato del mouse
  let staccata = false;  // si è scorso a mano: la pagina resta dov'è finché non si torna a scrivere
  let scorrimento = scrollY;  // dove l'abbiamo lasciata noi
  let timer: number | undefined;

  function centra(): void {
    testa = view.state.selection.head;
    if (!attiva()) return;
    const riga = view.coordsAtPos(testa);
    scrollBy({ top: (riga.top + riga.bottom) / 2 - innerHeight / 2, behavior: 'instant' });
    scorrimento = scrollY;
    staccata = false;
  }

  document.addEventListener('mousedown', () => { colMouse = true; }, true);
  document.addEventListener('keydown', () => { colMouse = false; }, true);
  addEventListener('scroll', () => { if (Math.abs(scrollY - scorrimento) > 1) staccata = true; }, { passive: true });

  return {
    centra,
    /** Da chiamare a ogni transazione dell'editor. */
    cambiato(docCambiato: boolean): void {
      const { head, empty } = view.state.selection;
      const mossa = head !== testa || selezione;
      testa = head;
      selezione = !empty;
      // con una selezione (Maiusc+frecce, Ctrl+A) la pagina non segue: si perderebbe il punto
      if (empty && (docCambiato || (mossa && !colMouse))) centra();
    },
    /** Le righe si sono spostate (zoom, font, finestra): se si stava scrivendo, si torna al centro. */
    ricentra(): void {
      clearTimeout(timer);
      timer = window.setTimeout(() => {
        if (!staccata && view.state.selection.empty) centra();
      }, ATTESA_COLONNA);
    },
  };
}
