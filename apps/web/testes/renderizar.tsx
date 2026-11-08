import { createMemoryHistory } from '@tanstack/react-router';
import { render } from '@testing-library/react';
import { criarCliente, type FonteDeEventos } from '@vidacor/api-client';
import { vi } from 'vitest';
import { Aplicacao, criarQueryClient } from '../src/Aplicacao';

/** Fonte SSE falsa: o teste decide quando abre, cai ou emite. */
export class FonteFalsa implements FonteDeEventos {
  readyState = 0;
  onopen: FonteDeEventos['onopen'] = null;
  onerror: FonteDeEventos['onerror'] = null;
  private readonly ouvintes = new Map<string, ((ev: MessageEvent<string>) => void)[]>();
  constructor(readonly url: string) {}
  addEventListener(tipo: string, ouvinte: (ev: MessageEvent<string>) => void): void {
    this.ouvintes.set(tipo, [...(this.ouvintes.get(tipo) ?? []), ouvinte]);
  }
  close(): void {
    this.readyState = 2;
  }
  abrir(): void {
    this.readyState = 1;
    this.onopen?.(new Event('open'));
  }
  cair(): void {
    this.readyState = 0;
    this.onerror?.(new Event('error'));
  }
  emitir(tipo: string, dados: unknown): void {
    for (const o of this.ouvintes.get(tipo) ?? []) {
      o(new MessageEvent(tipo, { data: JSON.stringify(dados) }));
    }
  }
}

export function renderizar(url: string) {
  const fontes: FonteFalsa[] = [];
  const irPara = vi.fn();
  const imprimir = vi.fn();
  const queryClient = criarQueryClient();
  queryClient.setDefaultOptions({ queries: { retry: false } });
  const tela = render(
    <Aplicacao
      queryClient={queryClient}
      history={createMemoryHistory({ initialEntries: [url] })}
      dependencias={{
        api: criarCliente({ esperar: () => Promise.resolve() }),
        irPara,
        imprimir,
        atrasoDoAvisoDeEventosMs: 0,
        atrasoDoRascunhoMs: 0,
        criarFonteDeEventos: (u) => {
          const f = new FonteFalsa(u);
          fontes.push(f);
          return f;
        },
      }}
    />,
  );
  return { ...tela, fontes, irPara, imprimir, queryClient };
}
