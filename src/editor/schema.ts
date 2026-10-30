import { Schema, type DOMOutputSpec } from 'prosemirror-model';

// Lo schema ammette solo ciò che il docx sa rappresentare: titolo, sottotitolo,
// paragrafo, grassetto e corsivo. Tutto il resto (anche se incollato) si perde qui,
// non nel file.
export const schema = new Schema({
  nodes: {
    doc: { content: 'block+' },

    // il paragrafo viene prima: è il blocco predefinito (Invio a fine titolo crea un paragrafo)
    paragraph: {
      group: 'block',
      content: 'text*',
      marks: '_',
      parseDOM: [{ tag: 'p' }],
      toDOM: (): DOMOutputSpec => ['p', 0],
    },

    // livello 1 = Titolo, livello 2 = Sottotitolo
    heading: {
      group: 'block',
      content: 'text*',
      marks: '_',
      defining: true,
      attrs: { level: { default: 1, validate: 'number' } },
      parseDOM: [
        { tag: 'h1', attrs: { level: 1 } },
        { tag: 'h2', attrs: { level: 2 } },
        { tag: 'h3', attrs: { level: 2 } },
        { tag: 'h4', attrs: { level: 2 } },
        { tag: 'h5', attrs: { level: 2 } },
        { tag: 'h6', attrs: { level: 2 } },
      ],
      toDOM: (node): DOMOutputSpec => ['h' + node.attrs.level, 0],
    },

    text: {},
  },

  marks: {
    strong: {
      parseDOM: [
        { tag: 'strong' },
        // Google Documenti avvolge tutto in <b style="font-weight:normal">
        { tag: 'b', getAttrs: node => node.style.fontWeight !== 'normal' && null },
        { style: 'font-weight=400', clearMark: m => m.type.name === 'strong' },
        { style: 'font-weight', getAttrs: value => /^(bold(er)?|[6-9]\d{2,})$/.test(value) && null },
      ],
      toDOM: (): DOMOutputSpec => ['strong', 0],
    },

    em: {
      parseDOM: [
        { tag: 'i' },
        { tag: 'em' },
        { style: 'font-style=italic' },
        { style: 'font-style=normal', clearMark: m => m.type.name === 'em' },
      ],
      toDOM: (): DOMOutputSpec => ['em', 0],
    },
  },
});
