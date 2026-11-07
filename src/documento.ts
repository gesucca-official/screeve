import type { Node } from 'prosemirror-model';
import { schema } from './editor/schema';

// Il modello condiviso con Rust (src-tauri/src/docx/mod.rs): ciò che finisce nel docx.
// `formato` è l'aspetto nel file aperto: per Screeve è opaco (vedi editor/schema.ts).
// La stringa vuota è testo del file senza formattazione propria; null è testo nuovo.
export interface Run { testo: string; grassetto: boolean; corsivo: boolean; sottolineato: boolean; barrato: boolean; formato?: string | null }
export interface Blocco { tipo: 'paragrafo' | 'titolo'; livello: number; run: Run[]; formato?: string | null }

export function daDoc(doc: Node): Blocco[] {
  const blocchi: Blocco[] = [];
  doc.forEach(node => {
    const titolo = node.type === schema.nodes.heading;
    const run: Run[] = [];
    node.forEach(t => run.push({
      testo: t.text ?? '',
      grassetto: schema.marks.strong.isInSet(t.marks) !== undefined,
      corsivo: schema.marks.em.isInSet(t.marks) !== undefined,
      sottolineato: schema.marks.underline.isInSet(t.marks) !== undefined,
      barrato: schema.marks.strike.isInSet(t.marks) !== undefined,
      formato: schema.marks.formato.isInSet(t.marks)?.attrs.xml ?? null,
    }));
    blocchi.push({ tipo: titolo ? 'titolo' : 'paragrafo', livello: titolo ? node.attrs.level : 0, run, formato: node.attrs.formato });
  });
  return blocchi;
}

export function aDoc(blocchi: Blocco[]): Node {
  const nodi = blocchi.map(b => {
    const testo = b.run.filter(r => r.testo).map(r => schema.text(r.testo, [
      ...(r.grassetto ? [schema.marks.strong.create()] : []),
      ...(r.corsivo ? [schema.marks.em.create()] : []),
      ...(r.sottolineato ? [schema.marks.underline.create()] : []),
      ...(r.barrato ? [schema.marks.strike.create()] : []),
      ...(r.formato != null ? [schema.marks.formato.create({ xml: r.formato })] : []),
    ]));
    const formato = b.formato ?? null;
    return b.tipo === 'titolo'
      ? schema.nodes.heading.create({ level: b.livello === 1 ? 1 : 2, formato }, testo)
      : schema.nodes.paragraph.create({ formato }, testo);
  });
  // lo schema vuole almeno un blocco
  return schema.nodes.doc.create(null, nodi.length ? nodi : schema.nodes.paragraph.create());
}

export const docVuoto = () => aDoc([]);
