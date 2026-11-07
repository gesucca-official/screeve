import { Schema, type DOMOutputSpec } from 'prosemirror-model';

// Lo schema ammette solo ciò che il docx sa rappresentare: titolo, sottotitolo,
// paragrafo, grassetto, corsivo, sottolineato e barrato. Tutto il resto (anche se incollato) si perde qui,
// non nel file.
// In più, invisibile: `formato` è l'aspetto che paragrafi e testo avevano nel file
// aperto (XML opaco per src-tauri/src/docx/modello.rs). Non si legge dal DOM, quindi
// non arriva mai da ciò che si incolla.
export const schema = new Schema({
  nodes: {
    doc: { content: 'block+' },

    // il paragrafo viene prima: è il blocco predefinito (Invio a fine titolo crea un paragrafo)
    paragraph: {
      group: 'block',
      content: 'text*',
      marks: '_',
      attrs: { formato: { default: null } },
      parseDOM: [{ tag: 'p' }],
      toDOM: (): DOMOutputSpec => ['p', 0],
    },

    // livello 1 = Titolo, livello 2 = Sottotitolo
    heading: {
      group: 'block',
      content: 'text*',
      marks: '_',
      defining: true,
      attrs: { level: { default: 1, validate: 'number' }, formato: { default: null } },
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

    underline: {
      parseDOM: [
        { tag: 'u' },
        { tag: 'ins' },
        { style: 'text-decoration', getAttrs: value => value.includes('underline') && null },
        { style: 'text-decoration-line', getAttrs: value => value.includes('underline') && null },
      ],
      toDOM: (): DOMOutputSpec => ['u', 0],
    },

    strike: {
      parseDOM: [
        { tag: 's' },
        { tag: 'strike' },
        { tag: 'del' },
        { style: 'text-decoration', getAttrs: value => value.includes('line-through') && null },
        { style: 'text-decoration-line', getAttrs: value => value.includes('line-through') && null },
      ],
      toDOM: (): DOMOutputSpec => ['s', 0],
    },

    formato: {
      attrs: { xml: {} },
      toDOM: (): DOMOutputSpec => ['span', 0],
    },
  },
});
