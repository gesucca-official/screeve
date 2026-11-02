import type { Node } from 'prosemirror-model';
import { TextSelection } from 'prosemirror-state';
import type { EditorView } from 'prosemirror-view';
import { schema } from './editor/schema';

// Indice a sinistra: i titoli e i sottotitoli del documento. Un clic ci porta il
// cursore; la voce evidenziata è la parte in cui si trova il cursore.
// Molti documenti non hanno titoli veri, solo righe formattate a mano: se non ce n'è
// nemmeno uno, l'indice le riconosce dall'aspetto (vedi riconosciute). Il documento
// non cambia.

interface Voce { pos: number; livello: number; testo: string }

const ATTESA = 200;     // ms dopo l'ultima modifica
const SOPRA = 90;       // px tra il titolo raggiunto e il bordo della finestra, per la barra
const RIGA_BREVE = 80;  // caratteri oltre i quali non è un titolo
const MAX_QUOTA = 0.25; // oltre questa quota di paragrafi non sono titoli, è un documento vario

// L'aspetto che distingue una riga: dal formato originale contano solo font, corpo,
// colore e maiuscoletto (la lingua, per esempio, cambia spesso e non si vede).
function aspetto(n: Node): { carattere: string; corpo: number; allineamento: string } {
  let piuLungo = n.firstChild;
  n.forEach(t => { if (piuLungo && t.nodeSize > piuLungo.nodeSize) piuLungo = t; });
  const xml: string = (piuLungo && schema.marks.formato.isInSet(piuLungo.marks)?.attrs.xml) || '';
  const valore = (re: RegExp, testo: string) => re.exec(testo)?.[1] ?? '';
  const carattere = [
    valore(/w:ascii(?:Theme)?="([^"]+)"/, xml),
    valore(/<w:sz w:val="(\d+)"/, xml),
    valore(/<w:color w:val="(\w+)"/, xml),
    /<w:(?:smallCaps|caps)(?: w:val="(?:1|true|on)")?\/>/.test(xml) ? 'maiuscoletto' : '',
  ].join('|');
  const allineamento = valore(/<w:jc w:val="(\w+)"/, n.attrs.formato ?? '').replace(/^(start|left)$/, '');
  return { carattere, corpo: +valore(/<w:sz w:val="(\d+)"/, xml), allineamento };
}

// Le righe che sembrano titoli: brevi, senza la punteggiatura di una frase, e diverse
// dal testo principale per font, corpo o allineamento centrato o a destra, oppure
// tutte in grassetto. Il solo corsivo non basta: in narrativa è troppo comune.
// Righe consecutive (luogo, ora) fanno una voce sola. Meglio nessuna voce che un
// indice sbagliato: se le righe sono troppe, non se ne mostra nessuna.
function riconosciute(doc: Node): Voce[] {
  const caratteri = new Map<string, number>();
  const paragrafi: { n: Node; pos: number; a: ReturnType<typeof aspetto> }[] = [];
  doc.forEach((n, pos) => {
    if (!n.textContent.trim()) return;
    const a = aspetto(n);
    paragrafi.push({ n, pos, a });
    const chiave = a.carattere + '/' + a.allineamento;
    caratteri.set(chiave, (caratteri.get(chiave) ?? 0) + n.textContent.length);
  });
  if (!paragrafi.length) return [];
  const principale = [...caratteri.entries()].reduce((x, y) => (y[1] > x[1] ? y : x))[0];
  const [carattere, allineamento] = principale.split('/');
  const corpo = paragrafi.find(p => p.a.carattere === carattere)!.a.corpo;

  const trovate: Voce[] = [];
  let ultimoIndice = -2;
  doc.forEach((n, pos, i) => {
    const testo = n.textContent.trim();
    if (!testo) { if (ultimoIndice === i - 1) ultimoIndice = i; return; }  // le righe vuote non separano
    if (testo.length > RIGA_BREVE || /[.,;:!?]$/.test(testo) || !/[\p{L}\p{N}]/u.test(testo)) return;
    const a = aspetto(n);
    const piuPiccola = a.corpo > 0 && corpo > 0 && a.corpo < corpo;
    const altroCarattere = a.carattere !== carattere && !piuPiccola;
    const altraPosizione = a.allineamento !== allineamento && /^(center|right|end)$/.test(a.allineamento);
    let grassetto = true;
    n.forEach(t => { if (!schema.marks.strong.isInSet(t.marks)) grassetto = false; });
    if (!altroCarattere && !altraPosizione && !grassetto) return;
    if (ultimoIndice === i - 1 && trovate.length) trovate[trovate.length - 1].testo += ' · ' + testo;
    else trovate.push({ pos, livello: 2, testo });
    ultimoIndice = i;
  });
  // le voci unite contano una riga per ognuna delle loro parti
  const righe = trovate.reduce((tot, v) => tot + v.testo.split(' · ').length, 0);
  return righe <= Math.max(1, paragrafi.length * MAX_QUOTA) ? trovate : [];
}

export function gestioneIndice(view: EditorView) {
  const lista = document.querySelector('#indice-lista') as HTMLElement;
  let voci: Voce[] = [];
  let firma = '';
  let stimate = false;  // voci riconosciute dall'aspetto, non titoli veri
  let timer: number | undefined;

  function leggi(doc: Node): Voce[] {
    const trovate: Voce[] = [];
    doc.forEach((n, pos) => {
      if (n.type === schema.nodes.heading && n.textContent.trim()) {
        trovate.push({ pos, livello: n.attrs.level, testo: n.textContent.trim() });
      }
    });
    return trovate;
  }

  function aggiorna(): void {
    voci = leggi(view.state.doc);
    stimate = !voci.length;
    if (stimate) voci = riconosciute(view.state.doc);
    // se cambiano solo le posizioni basta aggiornare i dati: la lista resta quella
    const nuova = JSON.stringify([stimate, voci.map(v => [v.livello, v.testo])]);
    if (nuova !== firma) {
      firma = nuova;
      disegna();
    }
    evidenzia();
  }

  function disegna(): void {
    if (!voci.length) {
      const vuoto = document.createElement('p');
      vuoto.textContent = 'I titoli del documento compariranno qui.';
      lista.replaceChildren(vuoto);
      return;
    }
    lista.replaceChildren(...voci.map((v, i) => {
      const b = document.createElement('button');
      b.textContent = v.testo;
      b.title = v.testo;
      b.className = stimate ? 'stimata' : `liv${v.livello}`;
      b.dataset.i = String(i);
      return b;
    }));
    if (stimate) {
      const nota = document.createElement('p');
      nota.textContent = 'Il documento non ha titoli: queste righe sono riconosciute dal loro aspetto.';
      lista.prepend(nota);
    }
  }

  function evidenzia(): void {
    const { from } = view.state.selection;
    let attuale = -1;
    voci.forEach((v, i) => { if (v.pos < from) attuale = i; });
    lista.querySelectorAll<HTMLElement>('button').forEach((b, i) => {
      const on = i === attuale;
      if (on && !b.classList.contains('on')) b.scrollIntoView({ block: 'nearest' });
      b.classList.toggle('on', on);
    });
  }

  lista.addEventListener('mousedown', e => e.preventDefault());  // il focus resta nel testo
  lista.addEventListener('click', e => {
    const b = (e.target as HTMLElement).closest<HTMLElement>('button');
    if (!b) return;
    aggiorna();  // l'ultima modifica potrebbe aver spostato i titoli
    const v = voci[+b.dataset.i!];
    if (!v) return;
    const { state } = view;
    view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, v.pos + 1)));
    view.focus();
    scrollBy({ top: view.coordsAtPos(v.pos + 1).top - SOPRA, behavior: 'smooth' });
  });

  return {
    /** Da chiamare a ogni transazione dell'editor. */
    cambiato(docCambiato: boolean): void {
      if (!docCambiato) { evidenzia(); return; }
      clearTimeout(timer);
      timer = setTimeout(aggiorna, ATTESA);
    },
    aggiorna,
  };
}
