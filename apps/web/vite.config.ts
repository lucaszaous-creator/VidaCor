import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin } from 'vite';

/**
 * CSP estrita na página publicada (SEC-CLI-2). Em `vite dev` não entra, porque o recarregamento do
 * Vite usa script embutido. `frame-ancestors` não vale em <meta>: o servidor que entrega a página
 * precisa mandar o cabeçalho `Content-Security-Policy` com `frame-ancestors 'none'`.
 */
function politicaDeSeguranca(): Plugin {
  const csp = [
    "default-src 'self'",
    "script-src 'self'",
    "style-src 'self'",
    "img-src 'self' data:",
    "font-src 'self'",
    "connect-src 'self'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'self'",
  ].join('; ');
  return {
    name: 'vidacor-csp',
    apply: 'build',
    transformIndexHtml: (html) =>
      html.replace('<meta charset="UTF-8" />', `<meta charset="UTF-8" />\n    <meta http-equiv="Content-Security-Policy" content="${csp}" />`),
  };
}

export default defineConfig({
  plugins: [react(), politicaDeSeguranca()],
  server: {
    port: 5173,
    strictPort: false,
    proxy: {
      // O núcleo Rust (axum) em desenvolvimento. Mesma origem para a página: cookie e SSE sem CORS.
      '/v1': { target: 'http://127.0.0.1:8080', changeOrigin: false },
    },
  },
  build: {
    sourcemap: false,
    rollupOptions: {
      output: {
        manualChunks: (id) => {
          if (!id.includes('node_modules')) {
            return undefined;
          }
          if (/[\\/](react-aria|@react-aria|@react-stately|@internationalized|@react-types)/.test(id)) {
            return 'react-aria';
          }
          if (id.includes('@tanstack')) {
            return 'tanstack';
          }
          return 'vendor';
        },
      },
    },
  },
});
