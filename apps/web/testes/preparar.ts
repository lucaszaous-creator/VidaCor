import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterAll, afterEach, beforeAll } from 'vitest';
import { servidor } from './nucleo-simulado';

// O jsdom não tem canvas; o axe tenta usá-lo e só registra erro no console. Sem canvas, null.
HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext;

// O roteador rola a página ao navegar; o jsdom não implementa a rolagem.
window.scrollTo = () => undefined;

beforeAll(() => {
  servidor.listen({ onUnhandledRequest: 'error' });
});
afterEach(() => {
  cleanup();
  servidor.resetHandlers();
});
afterAll(() => {
  servidor.close();
});
