import { Plugin, PluginKey, type EditorState } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

// I refusi sono decorazioni (sottolineatura ondulata), non marcature: non entrano
// mai nel documento né nel docx. Chi li calcola (src/correttore.ts) passa l'insieme
// completo con un meta; tra un controllo e l'altro le decorazioni seguono il testo.

export interface Intervallo { from: number; to: number }

interface Stato { set: DecorationSet; attuale: Intervallo | null }

export interface RefusiMeta {
  set?: DecorationSet;
  togli?: Intervallo;              // un refuso appena corretto, nelle posizioni nuove
  attuale?: Intervallo | null;     // quello con i suggerimenti aperti
}

export const refusiKey = new PluginKey<Stato>('refusi');

export const refuso = (from: number, to: number, parola: string) =>
  Decoration.inline(from, to, { class: 'refuso' }, { parola });

export function refusi(onClick: (r: Intervallo | null) => void): Plugin<Stato> {
  return new Plugin<Stato>({
    key: refusiKey,
    state: {
      init: () => ({ set: DecorationSet.empty, attuale: null }),
      apply(tr, prev) {
        const meta = tr.getMeta(refusiKey) as RefusiMeta | undefined;
        let set = meta?.set ?? prev.set.map(tr.mapping, tr.doc);
        if (meta?.togli) set = set.remove(set.find(meta.togli.from, meta.togli.to));
        const attuale = meta && 'attuale' in meta ? meta.attuale! : tr.docChanged ? null : prev.attuale;
        return { set, attuale };
      },
    },
    props: {
      decorations(state) {
        const { set, attuale } = refusiKey.getState(state)!;
        let visibili = set;
        // la parola che stai scrivendo non si sottolinea finché il cursore ci resta attaccato
        const { selection } = state;
        if (selection.empty) {
          const qui = set.find(selection.from, selection.from).filter(d => d.to === selection.from);
          if (qui.length) visibili = visibili.remove(qui);
        }
        if (attuale) visibili = visibili.add(state.doc, [Decoration.inline(attuale.from, attuale.to, { class: 'refuso-attuale' })]);
        return visibili;
      },
      handleClick(view, pos) {
        const d = refusiKey.getState(view.state)!.set.find(pos, pos).find(d => d.from < pos && pos < d.to);
        onClick(d ? { from: d.from, to: d.to } : null);
        return false;
      },
    },
  });
}

// Tutti i refusi in ordine, anche quello nascosto sotto il cursore.
export function elencoRefusi(state: EditorState): (Intervallo & { parola: string })[] {
  return refusiKey.getState(state)!.set.find()
    .map(d => ({ from: d.from, to: d.to, parola: d.spec.parola as string }))
    .sort((a, b) => a.from - b.from);
}
