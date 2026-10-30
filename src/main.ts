import '@fontsource-variable/literata/opsz.css';
import '@fontsource-variable/literata/opsz-italic.css';
import './styles/theme.css';
import './styles/app.css';

const editor = document.getElementById('editor') as HTMLElement;
const count = document.getElementById('count') as HTMLElement;

const fmt = (n: number) => n.toLocaleString('it-IT');

// "dell'acqua" conta come una parola, come in Word
const WORD = /[\p{L}\p{N}]+(?:['’][\p{L}\p{N}]+)*/gu;

function updateCount(): void {
  const text = editor.innerText;
  const words = text.match(WORD)?.length ?? 0;
  const chars = text.replace(/\n/g, '').length;
  count.textContent = `${fmt(words)} parole · ${fmt(chars)} caratteri`;
}

let timer: number | undefined;
editor.addEventListener('input', () => {
  clearTimeout(timer);
  timer = window.setTimeout(updateCount, 300);
});

// Fuori dal testo il menu contestuale del browser (Indietro, Aggiorna…) non ha senso.
document.addEventListener('contextmenu', e => {
  if (!editor.contains(e.target as Node)) e.preventDefault();
});

document.execCommand('defaultParagraphSeparator', false, 'p');
updateCount();
editor.focus();
