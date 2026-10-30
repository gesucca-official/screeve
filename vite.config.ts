import { defineConfig } from 'vite';

// Configurazione pensata per Tauri: porta fissa e nessun watch sulla parte Rust.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    target: 'chrome110', // WebView2 su Windows 10/11
    outDir: 'dist',
  },
});
