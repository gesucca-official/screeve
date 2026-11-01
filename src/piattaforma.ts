// Differenze tra Windows e macOS che riguardano l'interfaccia: il tasto delle
// scorciatoie (Ctrl o ⌘) e come le scorciatoie vengono scritte a schermo.

export const mac = /Mac/i.test(navigator.platform);

/** Il tasto "comando" della piattaforma: ⌘ su macOS, Ctrl altrove. */
export const modificatore = (e: KeyboardEvent | MouseEvent | WheelEvent): boolean => (mac ? e.metaKey : e.ctrlKey);

/** Scrive una scorciatoia come si usa sulla piattaforma: «Ctrl+Alt+1» diventa «⌘⌥1» su macOS. */
export function tasti(testo: string): string {
  if (!mac) return testo;
  return testo
    .replace(/\bCtrl\b/g, '⌘')
    .replace(/\bAlt\b/g, '⌥')
    .replace(/\bMaiusc\b/g, '⇧')
    .replace(/([⌘⌥⇧])\+/g, '$1');
}

/** Adatta a macOS le scorciatoie già scritte nella pagina (tooltip, menu, guida). */
export function adattaScorciatoie(radice: ParentNode = document): void {
  if (!mac) return;
  radice.querySelectorAll<HTMLElement>('[title]').forEach(el => { el.title = tasti(el.title); });
  radice.querySelectorAll('kbd').forEach(k => {
    const prima = k.textContent ?? '';
    const dopo = tasti(prima);
    if (dopo === prima) return;
    k.textContent = dopo;
    // «<kbd>Ctrl</kbd>+<kbd>N</kbd>»: il «+» tra due tasti non serve più
    const seguente = k.nextSibling;
    if (/^[⌘⌥⇧]$/.test(dopo) && seguente?.nodeType === Node.TEXT_NODE) {
      seguente.textContent = seguente.textContent!.replace(/^\+/, '');
    }
  });
}
