import { InputRule, inputRules } from 'prosemirror-inputrules';
import type { Plugin } from 'prosemirror-state';

export interface AutocorrectOptions {
  enabled: () => boolean;            // letto a ogni battuta: si attiva e disattiva dalle impostazioni
  onApply: (label: string) => void;  // per il messaggio nella barra di stato
}

// Correzioni automatiche mentre si scrive. Ognuna è un singolo passo:
// Ctrl+Z (o Backspace subito dopo) riporta il testo com'era stato digitato.
export function autocorrect({ enabled, onApply }: AutocorrectOptions): Plugin {
  const rule = (match: RegExp, label: string, replace: (m: RegExpMatchArray) => string) =>
    new InputRule(match, (state, m, start, end) => {
      if (!enabled()) return null;
      onApply(label);
      // il testo appena digitato non è ancora nel documento: sostituiamo fino a "end"
      return state.tr.insertText(replace(m), start, end);
    });

  return inputRules({
    rules: [
      rule(/<<$/, '<< → «', () => '«'),
      rule(/>>$/, '>> → »', () => '»'),
      // E' all'inizio di una parola, seguito da spazio o punteggiatura
      rule(/(?<=^|[\s«"‹(—])E['’]([\s.,;:!?»›)])$/, "E' → È", m => 'È' + m[1]),
    ],
  });
}
