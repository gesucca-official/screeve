import type { Node } from 'prosemirror-model';
import { schema } from './editor/schema';

// Il modello condiviso con Rust (src-tauri/src/docx/mod.rs): ciò che finisce nel docx.
export interface Run { testo: string; grassetto: boolean; corsivo: boolean }
export interface Blocco { tipo: 'paragrafo' | 'titolo'; livello: number; run: Run[] }

export function daDoc(doc: Node): Blocco[] {
  const blocchi: Blocco[] = [];
  doc.forEach(node => {
    const titolo = node.type === schema.nodes.heading;
    const run: Run[] = [];
    node.forEach(t => run.push({
      testo: t.text ?? '',
      grassetto: schema.marks.strong.isInSet(t.marks) !== undefined,
      corsivo: schema.marks.em.isInSet(t.marks) !== undefined,
    }));
    blocchi.push({ tipo: titolo ? 'titolo' : 'paragrafo', livello: titolo ? node.attrs.level : 0, run });
  });
  return blocchi;
}

export function aDoc(blocchi: Blocco[]): Node {
  const nodi = blocchi.map(b => {
    const testo = b.run.filter(r => r.testo).map(r => schema.text(r.testo, [
      ...(r.grassetto ? [schema.marks.strong.create()] : []),
      ...(r.corsivo ? [schema.marks.em.create()] : []),
    ]));
    return b.tipo === 'titolo'
      ? schema.nodes.heading.create({ level: b.livello === 1 ? 1 : 2 }, testo)
      : schema.nodes.paragraph.create(null, testo);
  });
  // lo schema vuole almeno un blocco
  return schema.nodes.doc.create(null, nodi.length ? nodi : schema.nodes.paragraph.create());
}

export const docVuoto = () => aDoc([]);
