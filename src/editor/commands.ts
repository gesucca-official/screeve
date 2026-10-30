import { setBlockType, toggleMark } from 'prosemirror-commands';
import type { MarkType } from 'prosemirror-model';
import { Selection, TextSelection, type Command, type EditorState } from 'prosemirror-state';
import { schema } from './schema';

export type Block = 'h1' | 'h2' | 'p';

// Il tipo del blocco in cui sta il cursore (o l'inizio della selezione).
export function currentBlock(state: EditorState): Block {
  const node = state.selection.$from.parent;
  if (node.type === schema.nodes.heading) return node.attrs.level === 1 ? 'h1' : 'h2';
  return 'p';
}

// Titolo e Sottotitolo sono interruttori: ripremere torna a Testo.
export function setBlock(block: Block): Command {
  return (state, dispatch) => {
    const target = block !== 'p' && currentBlock(state) === block ? 'p' : block;
    return target === 'p'
      ? setBlockType(schema.nodes.paragraph)(state, dispatch)
      : setBlockType(schema.nodes.heading, { level: target === 'h1' ? 1 : 2 })(state, dispatch);
  };
}

export const toggleStrong = toggleMark(schema.marks.strong);
export const toggleEm = toggleMark(schema.marks.em);

// Il mark è attivo dove sta il cursore, o su tutta la selezione?
export function markActive(state: EditorState, type: MarkType): boolean {
  const { from, to, empty, $from } = state.selection;
  if (empty) return !!type.isInSet(state.storedMarks || $from.marks());
  return state.doc.rangeHasMark(from, to, type);
}

export function insertChar(text: string): Command {
  return (state, dispatch) => {
    dispatch?.(state.tr.insertText(text).scrollIntoView());
    return true;
  };
}

// Racchiude la selezione tra « »; senza selezione inserisce «» col cursore in mezzo.
export const wrapGuillemets: Command = (state, dispatch) => {
  const { empty } = state.selection;
  if (!dispatch) return true;
  // con Ctrl+A la selezione parte e finisce fuori dai blocchi: la riportiamo dentro al testo
  const from = Selection.findFrom(state.selection.$from, 1, true)?.from ?? state.selection.from;
  let to = Selection.findFrom(state.selection.$to, -1, true)?.to ?? state.selection.to;
  if (empty) {
    const tr = state.tr.insertText('«»');
    dispatch(tr.setSelection(TextSelection.create(tr.doc, from + 1)).scrollIntoView());
    return true;
  }
  // il doppio clic su Windows include lo spazio finale: lo lasciamo fuori
  while (to > from && state.doc.textBetween(to - 1, to) === ' ') to--;
  const tr = state.tr.insertText('»', to).insertText('«', from);
  dispatch(tr.setSelection(TextSelection.create(tr.doc, from + 1, to + 1)).scrollIntoView());
  return true;
};
