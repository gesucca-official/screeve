import { setBlockType, toggleMark } from 'prosemirror-commands';
import type { MarkType } from 'prosemirror-model';
import { Selection, TextSelection, type Command, type EditorState, type Transaction } from 'prosemirror-state';
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

// Sostituisce una parola (refuso corretto, sinonimo): la nuova eredita grassetto,
// corsivo e maiuscola iniziale, e il cursore va subito dopo.
export function replaceWord(state: EditorState, from: number, to: number, word: string): Transaction {
  if (/^\p{Lu}/u.test(state.doc.textBetween(from, to))) word = word.charAt(0).toUpperCase() + word.slice(1);
  const marks = state.doc.nodeAt(from)?.marks ?? [];
  const tr = state.tr.replaceWith(from, to, schema.text(word, marks));
  return tr.setSelection(TextSelection.create(tr.doc, from + word.length));
}

// La parola selezionata, o quella sotto il cursore. In "dell'acqua" le parole sono
// due: l'articolo eliso e "acqua".
export function wordAt(state: EditorState): { from: number; to: number; text: string } | null {
  const { $from, from, to, empty } = state.selection;
  if (!$from.parent.isTextblock) return null;
  const start = $from.start();
  const text = $from.parent.textContent;
  const word = /\p{L}+/gu;
  if (!empty) {
    const sel = state.doc.textBetween(from, to).trim();
    const at = state.doc.textBetween(from, to).indexOf(sel);
    return /^\p{L}+$/u.test(sel) && $from.sameParent(state.selection.$to)
      ? { from: from + at, to: from + at + sel.length, text: sel } : null;
  }
  for (const m of text.matchAll(word)) {
    const a = start + m.index, b = a + m[0].length;
    if (a <= from && from <= b) return { from: a, to: b, text: m[0] };
  }
  return null;
}
