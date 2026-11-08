import { createContext, useContext } from 'react';
import type { ClienteVidaCor, DocumentoBaixado, FonteDeEventos } from '@vidacor/api-client';

/** Dependências da aplicação, trocáveis nos testes (núcleo simulado, SSE falso). */
export interface Dependencias {
  api: ClienteVidaCor;
  criarFonteDeEventos?: (url: string) => FonteDeEventos;
  /** Quanto esperar o SSE voltar sozinho antes de avisar que caiu (padrão 10 s; testes usam 0). */
  atrasoDoAvisoDeEventosMs?: number;
  /** Navegação de página inteira (entrada OIDC e reautenticação no servidor). */
  irPara: (url: string) => void;
  /** Impressão da página (pulseira). Padrão: `window.print()`; testes trocam por um espião. */
  imprimir?: () => void;
  /** Entrega de arquivo baixado (PDF). Padrão: download pelo navegador; testes trocam por um espião. */
  salvarArquivo?: (doc: DocumentoBaixado) => void;
  /** Espera depois da última tecla antes de salvar o rascunho (padrão 1,2 s; testes usam 0). */
  atrasoDoRascunhoMs?: number;
  /** Intervalo da consulta do recebimento até o PIX e o pagamento chegarem (padrão 2 s). */
  intervaloDoRecebimentoMs?: number;
}

export const ContextoDeDependencias = createContext<Dependencias | null>(null);

export function useDependencias(): Dependencias {
  const d = useContext(ContextoDeDependencias);
  if (d === null) {
    throw new Error('ContextoDeDependencias ausente');
  }
  return d;
}
