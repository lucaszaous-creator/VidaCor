import { useQueryClient } from '@tanstack/react-query';
import { abrirEventos, type EstadoDosEventos, type EventoEstudoRecebido } from '@vidacor/api-client';
import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { chaves } from '../consultas';
import { useDependencias } from '../contexto';

export interface EventosAoVivo {
  estado: EstadoDosEventos;
  /** O tempo real caiu e não voltou sozinho dentro do prazo: hora de avisar na tela. */
  foraDoAr: boolean;
  /** Estudos recebidos nesta sessão da tela, o mais recente primeiro (só em memória). */
  recebidos: EventoEstudoRecebido[];
}

const Contexto = createContext<EventosAoVivo>({ estado: 'conectando', foraDoAr: false, recebidos: [] });

export function useEventosAoVivo(): EventosAoVivo {
  return useContext(Contexto);
}

/**
 * Abre `GET /v1/eventos` (SSE) enquanto há sessão. Cada estudo recebido invalida a agenda e o
 * atendimento no cache: a tela busca de novo no servidor em vez de montar o dado sozinha.
 */
export function ProvedorDeEventos({ children }: { children: ReactNode }) {
  const { api, criarFonteDeEventos, atrasoDoAvisoDeEventosMs = 10_000 } = useDependencias();
  const queryClient = useQueryClient();
  const [estado, setEstado] = useState<EstadoDosEventos>('conectando');
  const [recebidos, setRecebidos] = useState<EventoEstudoRecebido[]>([]);
  const [quedaLonga, setQuedaLonga] = useState(false);
  const foraDoAr = estado === 'reconectando' && quedaLonga;

  // O navegador reconecta o SSE sozinho em segundos; só avisa se a queda durar.
  useEffect(() => {
    if (estado !== 'reconectando') {
      return undefined;
    }
    const id = setTimeout(() => {
      setQuedaLonga(true);
    }, atrasoDoAvisoDeEventosMs);
    return () => {
      clearTimeout(id);
    };
  }, [estado, atrasoDoAvisoDeEventosMs]);

  useEffect(() => {
    const conexao = abrirEventos({
      url: api.urlDeEventos(),
      ...(criarFonteDeEventos === undefined ? {} : { criarFonte: criarFonteDeEventos }),
      aoMudarEstado: (novo) => {
        setEstado(novo);
        if (novo === 'conectado') {
          setQuedaLonga(false);
        }
      },
      aoEstudoRecebido: (evento) => {
        setRecebidos((anteriores) => [evento, ...anteriores].slice(0, 10));
        void queryClient.invalidateQueries({ queryKey: chaves.agendaTodas });
        void queryClient.invalidateQueries({ queryKey: chaves.atendimento(evento.atendimento_id) });
      },
    });
    return () => {
      conexao.fechar();
    };
  }, [api, criarFonteDeEventos, queryClient]);

  return <Contexto.Provider value={{ estado, foraDoAr, recebidos }}>{children}</Contexto.Provider>;
}
