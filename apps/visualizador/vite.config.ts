import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// A CSP do programa fica no tauri.conf.json (app.security.csp), aplicada pelo Tauri a toda página.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5174, strictPort: true },
  build: { sourcemap: false, target: 'es2022' },
});
