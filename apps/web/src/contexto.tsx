import { createContext, useContext } from 'react';
import type { ClienteVidaCor, FonteDeEventos } from '@vidacor/api-client';

/** Dependências da aplicação, trocáveis nos testes (núcleo simulado, SSE falso). */
export interface Dependencias {
  api: ClienteVidaCor;
  criarFonteDeEventos?: (url: string) => FonteDeEventos;
  /** Navegação de página inteira (entrada OIDC no servidor). */
  irPara: (url: string) => void;
}

export const ContextoDeDependencias = createContext<Dependencias | null>(null);

export function useDependencias(): Dependencias {
  const d = useContext(ContextoDeDependencias);
  if (d === null) {
    throw new Error('ContextoDeDependencias ausente');
  }
  return d;
}
