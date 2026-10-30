import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { Node } from 'prosemirror-model';
import type { EditorView } from 'prosemirror-view';
import { chiedi } from './dialogo';
import { aDoc, daDoc, docVuoto, type Blocco } from './documento';

// Nuovo, Apri, Salva e Salva con nome; nome del file e stato salvato/modificato
// nella barra in alto e nel titolo della finestra. La lettura e la scrittura del
// docx sono in Rust (src-tauri/src/docx).
// Con modifiche non salvate, una copia di sicurezza va in Rust (recupero.rs) poco
// dopo l'ultima modifica; dopo un crash, all'avvio si propone di riaprirla.

interface Apertura { blocchi: Blocco[]; nonSupportati: { elemento: string; quanti: number }[] }
interface Copia { percorso: string | null; nome: string; daSalvare: boolean; blocchi: Blocco[]; quando: number }

const ATTESA_COPIA = 2000;  // ms dopo l'ultima modifica

const FILTRO = { name: 'Documento Word', extensions: ['docx'] };

// singolare e plurale degli elementi che Screeve non gestisce (chiavi da lettura.rs)
const ELEMENTI: Record<string, [string, string]> = {
  tabelle: ['tabella', 'tabelle'],
  immagini: ['immagine', 'immagini'],
  caselle: ['casella di testo', 'caselle di testo'],
  oggetti: ['oggetto incorporato', 'oggetti incorporati'],
  formule: ['formula', 'formule'],
  note: ['nota', 'note'],
  commenti: ['commento', 'commenti'],
  revisioni: ['revisione', 'revisioni'],
  elenchi: ['voce di elenco', 'voci di elenco'],
  intestazioni: ['intestazione o piè di pagina', 'intestazioni e piè di pagina'],
  interruzioni: ['interruzione di pagina', 'interruzioni di pagina'],
  campi: ['campo', 'campi'],
  collegamenti: ['collegamento', 'collegamenti'],
  sottolineature: ['parte sottolineata o barrata', 'parti sottolineate o barrate'],
  simboli: ['simbolo', 'simboli'],
};

export interface FileOptions {
  view: EditorView;
  flash: (testo: string) => void;
  sostituisci: (doc: Node) => void;  // mette il documento nell'editor
}

export function gestioneFile({ view, flash, sostituisci }: FileOptions) {
  const app = isTauri();
  let percorso: string | null = null;
  let nome = 'Senza titolo';
  let salvato: Node = view.state.doc;  // il documento com'era all'ultimo salvataggio
  let daSalvare = false;               // copia di un file con elementi persi: non esiste ancora su disco
  let modificato = false;
  let occupato = false;
  let mostrato = '';
  let copiaScritta = false;
  let timerCopia: number | undefined;

  const titolo = document.querySelector('#file-name') as HTMLElement;

  function aggiorna(): void {
    modificato = daSalvare || !view.state.doc.eq(salvato);
    if (app) {
      clearTimeout(timerCopia);
      timerCopia = setTimeout(copia, modificato ? ATTESA_COPIA : 0);
    }
    const stato = percorso && !daSalvare ? (modificato ? 'modificato' : 'salvato') : (modificato ? 'non salvato' : '');
    const chiave = nome + '|' + stato;
    if (chiave === mostrato) return;
    mostrato = chiave;
    const small = document.createElement('small');
    small.textContent = stato;
    titolo.replaceChildren(nome, small);
    titolo.title = percorso ?? '';
    const finestra = `${modificato ? '• ' : ''}${nome} — Screeve`;
    if (app) getCurrentWindow().setTitle(finestra).catch(() => {});
    else document.title = finestra;
  }

  /* ---------- copia di sicurezza ---------- */
  function copia(): void {
    if (modificato) {
      const c: Copia = { percorso, nome, daSalvare, blocchi: daDoc(view.state.doc), quando: Date.now() };
      invoke('salva_copia', { copia: c }).then(() => { copiaScritta = true; }, e => flash(String(e)));
    } else if (copiaScritta) {
      copiaScritta = false;
      invoke('cancella_copia').catch(() => {});
    }
  }

  // Dopo un crash: la copia più recente. Esc la lascia lì per la prossima volta.
  async function recupera(): Promise<void> {
    const [orfana] = await invoke<{ id: string; copia: Copia }[]>('copie_orfane');
    if (!orfana) return;
    const c = orfana.copia;
    const quando = new Date(c.quando).toLocaleString('it-IT', { weekday: 'long', day: 'numeric', month: 'long', hour: '2-digit', minute: '2-digit' });
    const scelta = await chiedi(
      `Screeve si è chiuso senza salvare «${c.nome}»`,
      `C'è una copia delle modifiche di ${quando}. Il file originale non è stato toccato.`,
      [{ etichetta: 'Riapri le modifiche', valore: 'riapri' }, { etichetta: 'Scartale', valore: 'scarta' }],
      'dopo',
    );
    if (scelta === 'dopo') return;
    if (scelta === 'riapri') {
      carica(aDoc(c.blocchi), c.percorso);
      nome = c.nome;
      daSalvare = c.daSalvare;
      salvato = docVuoto();  // tutto il testo recuperato è da salvare
      aggiorna();
    }
    await invoke('dimentica_copia', { id: orfana.id });
  }

  function carica(doc: Node, da: string | null): void {
    sostituisci(doc);
    percorso = da;
    nome = da ? baseName(da) : 'Senza titolo';
    salvato = view.state.doc;
    daSalvare = false;
    aggiorna();
  }

  // Un'operazione alla volta (Ctrl+S tenuto premuto, doppio clic sulla X…).
  function una<A extends unknown[], R>(f: (...a: A) => Promise<R>, altrimenti: R) {
    return async (...a: A): Promise<R> => {
      if (occupato) return altrimenti;
      if (!app) { flash('Apri e salva funzionano nell\'app, non nel browser'); return altrimenti; }
      occupato = true;
      try { return await f(...a); } finally { occupato = false; view.focus(); }
    };
  }

  /* ---------- modifiche non salvate ---------- */
  async function conferma(): Promise<boolean> {
    if (!modificato) return true;
    const scelta = await chiedi(
      daSalvare ? `Vuoi salvare «${nome}»?` : `Vuoi salvare le modifiche a «${nome}»?`,
      'Se non salvi, le modifiche andranno perse.',
      [{ etichetta: 'Salva', valore: 'salva' }, { etichetta: 'Non salvare', valore: 'no' }, { etichetta: 'Annulla', valore: 'annulla' }],
      'annulla',
    );
    if (scelta === 'salva') return salvaQui();
    return scelta === 'no';
  }

  /* ---------- salvataggio ---------- */
  async function scrivi(dove: string): Promise<boolean> {
    const doc = view.state.doc;
    try {
      await invoke('salva_docx', { percorso: dove, blocchi: daDoc(doc) });
    } catch (e) {
      await errore(e);
      return false;
    }
    percorso = dove;
    nome = baseName(dove);
    salvato = doc;  // se nel frattempo hai scritto ancora, resta "modificato"
    daSalvare = false;
    aggiorna();
    flash('Salvato');
    return true;
  }

  async function salvaQui(): Promise<boolean> {
    return percorso && !daSalvare ? scrivi(percorso) : salvaConNomeQui();
  }

  async function salvaConNomeQui(): Promise<boolean> {
    const scelto = await save({ filters: [FILTRO], defaultPath: suggerito() });
    return scelto ? scrivi(/\.docx$/i.test(scelto) ? scelto : scelto + '.docx') : false;
  }

  function suggerito(): string {
    if (percorso) return percorso;
    // senza file, il primo titolo è un buon nome
    const primo = view.state.doc.firstChild;
    const testo = primo?.type.name === 'heading' ? primo.textContent : '';
    const pulito = testo.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '').replace(/\s+/g, ' ').trim().slice(0, 80).replace(/[. ]+$/, '');
    return (pulito || 'Senza titolo') + '.docx';
  }

  /* ---------- apertura ---------- */
  async function apriQui(da?: string): Promise<void> {
    if (!(await conferma())) return;
    const scelto = da ?? await open({ multiple: false, directory: false, filters: [FILTRO], defaultPath: cartella(percorso) });
    if (!scelto) return;
    let letto: Apertura;
    try {
      letto = await invoke<Apertura>('apri_docx', { percorso: scelto });
    } catch (e) {
      await errore(e);
      return;
    }
    carica(aDoc(letto.blocchi), scelto);
    if (letto.nonSupportati.length) await avvisa(letto.nonSupportati, scelto);
  }

  // L'originale non si sovrascrive per sbaglio: la scelta predefinita è salvarne una copia.
  async function avvisa(elementi: Apertura['nonSupportati'], originale: string): Promise<void> {
    const elenco = new Intl.ListFormat('it', { type: 'conjunction' }).format(
      elementi.map(({ elemento, quanti }) => {
        const [uno, tanti] = ELEMENTI[elemento] ?? [elemento, elemento];
        return `${quanti} ${quanti === 1 ? uno : tanti}`;
      }));
    const scelta = await chiedi(
      'Questo documento contiene elementi che Screeve non gestisce',
      `${maiuscola(elenco)}. Salvando andrebbero persi: per lasciare intatto l'originale, salva una copia.`,
      [{ etichetta: 'Salva una copia', valore: 'copia' }, { etichetta: 'Modifica l\'originale', valore: 'originale' }],
      'copia-dopo',
    );
    if (scelta === 'originale') return;
    // da qui in poi è una copia senza file: Salva chiede dove metterla
    percorso = originale.replace(/(\.docx)?$/i, ' (copia).docx');
    nome = baseName(percorso);
    daSalvare = true;
    aggiorna();
    if (scelta === 'copia') await salvaConNomeQui();
  }

  async function errore(e: unknown): Promise<void> {
    // i messaggi di Rust sono "Non riesco a…: motivo"
    const testo = String(e);
    const i = testo.indexOf(': ');
    const [domanda, motivo] = i > 0 ? [testo.slice(0, i), maiuscola(testo.slice(i + 2)) + '.'] : [testo, ''];
    await chiedi(domanda, motivo, [{ etichetta: 'OK', valore: null }], null);
  }

  const nuovoQui = async () => {
    if (await conferma()) carica(docVuoto(), null);
  };

  /* ---------- avvio ---------- */
  function avvia(): void {
    aggiorna();
    if (!app) {
      addEventListener('beforeunload', e => { if (modificato) e.preventDefault(); });
      return;
    }
    const finestra = getCurrentWindow();
    let chiusura = false;
    finestra.onCloseRequested(async e => {
      if (!modificato) return;  // Tauri chiude da sé
      e.preventDefault();
      if (chiusura) return;
      chiusura = true;
      try {
        if (await conferma()) await finestra.destroy();
      } finally { chiusura = false; view.focus(); }
    });
    // un .docx trascinato sulla finestra
    getCurrentWebview().onDragDropEvent(e => {
      if (e.payload.type !== 'drop') return;
      const docx = e.payload.paths.find(p => /\.docx$/i.test(p));
      if (docx) apri(docx);
      else if (e.payload.paths.length) flash('Screeve apre solo file .docx');
    });
    // prima l'eventuale recupero, poi "Apri con", doppio clic su un .docx o
    // trascinamento sull'eseguibile (che chiede se salvare ciò che si è recuperato)
    recupera()
      .catch(e => flash(String(e)))
      .then(() => invoke<string | null>('file_iniziale'))
      .then(p => { if (p) apri(p); });
  }

  const apri = una(apriQui, undefined);
  return {
    nuovo: una(nuovoQui, undefined),
    apri,
    salva: una(salvaQui, false),
    salvaConNome: una(salvaConNomeQui, false),
    aggiorna,
    avvia,
  };
}

const baseName = (p: string) => p.split(/[\\/]/).pop() || p;
const cartella = (p: string | null) => p?.replace(/[\\/][^\\/]*$/, '') || undefined;
const maiuscola = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
