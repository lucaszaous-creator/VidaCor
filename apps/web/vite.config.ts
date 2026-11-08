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
    // O React Aria injeta um único <style> (usePress, seleção de texto). Liberado só pelo hash do
    // conteúdo, sem 'unsafe-inline'. Ao atualizar o react-aria-components, conferir no navegador
    // se o hash mudou (o console mostra a violação com o hash novo).
    "style-src 'self' 'sha256-38RhXrc7EdReTKsOm23ZPOCUgniTUUcjky8QOOrQx6o='",
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
    // Aplicação de rede local, carregada uma vez por turno: um pacote único de ~550 kB (180 kB
    // comprimido) é aceitável. Dividir à mão quebrou a ordem de inicialização dos módulos.
    chunkSizeWarningLimit: 700,
  },
});
