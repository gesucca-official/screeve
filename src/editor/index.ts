import { baseKeymap, chainCommands } from 'prosemirror-commands';
import { history, redo, undo } from 'prosemirror-history';
import { undoInputRule } from 'prosemirror-inputrules';
import { keymap } from 'prosemirror-keymap';
import type { Node } from 'prosemirror-model';
import { EditorState, Plugin } from 'prosemirror-state';
import { Decoration, DecorationSet, EditorView } from 'prosemirror-view';
import { autocorrect, type AutocorrectOptions } from './autocorrect';
import { setBlock, toggleEm, toggleStrong, wrapGuillemets } from './commands';
import { refusi, type Intervallo } from './refusi';
import { schema } from './schema';

export * from './commands';
export * from './refusi';

// Documento vuoto: un invito, non del testo.
const placeholder = new Plugin({
  props: {
    decorations(state) {
      const { doc } = state;
      const only = doc.firstChild;
      if (doc.childCount !== 1 || !only || only.content.size > 0) return null;
      return DecorationSet.create(doc, [
        Decoration.node(0, only.nodeSize, { class: 'empty', 'data-placeholder': 'Comincia a scrivere…' }),
      ]);
    },
  },
});

export interface EditorOptions extends AutocorrectOptions {
  onChange: (view: EditorView) => void;  // documento o selezione cambiati
  onTypoClick: (r: Intervallo | null) => void;  // clic su un refuso, o altrove nel testo
}

export function createEditor(mount: HTMLElement, options: EditorOptions): EditorView {
  const state = EditorState.create({
    schema,
    plugins: [
      autocorrect(options),
      history(),
      keymap({
        'Mod-z': chainCommands(undoInputRule, undo),
        'Mod-y': redo,
        'Shift-Mod-z': redo,
        'Backspace': undoInputRule,
        'Mod-b': toggleStrong,
        'Mod-i': toggleEm,
        'Mod-Alt-1': setBlock('h1'),
        'Mod-Alt-2': setBlock('h2'),
        'Mod-Alt-0': setBlock('p'),
        // Ctrl+Maiusc+': sulla tastiera italiana l'apostrofo maiuscolo è "?"
        "Shift-Mod-'": wrapGuillemets,
        'Shift-Mod-?': wrapGuillemets,
      }),
      keymap(baseKeymap),
      placeholder,
      refusi(r => options.onTypoClick(r)),
    ],
  });

  const view: EditorView = new EditorView({ mount }, {
    state,
    attributes: { spellcheck: 'false', 'aria-label': 'Testo' },
    dispatchTransaction(tr) {
      view.updateState(view.state.apply(tr));
      options.onChange(view);
    },
  });
  return view;
}

// Sostituisce il documento (Nuovo, Apri) col cursore all'inizio. Lo stato è nuovo,
// quindi anche la cronologia di Annulla riparte da qui. Non passa da onChange.
export function replaceDoc(view: EditorView, doc: Node): void {
  view.updateState(EditorState.create({ doc, plugins: view.state.plugins }));
}
