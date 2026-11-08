import { useQueryClient } from '@tanstack/react-query';
import { abrirEventos, type EstadoDosEventos, type EventoEstudoRecebido } from '@vidacor/api-client';
import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { chaves } from '../consultas';
import { useDependencias } from '../contexto';

export interface EventosAoVivo {
  estado: EstadoDosEventos;
  /** Estudos recebidos nesta sessão da tela, o mais recente primeiro (só em memória). */
  recebidos: EventoEstudoRecebido[];
}

const Contexto = createContext<EventosAoVivo>({ estado: 'conectando', recebidos: [] });

export function useEventosAoVivo(): EventosAoVivo {
  return useContext(Contexto);
}

/**
 * Abre `GET /v1/eventos` (SSE) enquanto há sessão. Cada estudo recebido invalida a agenda e o
 * atendimento no cache: a tela busca de novo no servidor em vez de montar o dado sozinha.
 */
export function ProvedorDeEventos({ children }: { children: ReactNode }) {
  const { api, criarFonteDeEventos } = useDependencias();
  const queryClient = useQueryClient();
  const [estado, setEstado] = useState<EstadoDosEventos>('conectando');
  const [recebidos, setRecebidos] = useState<EventoEstudoRecebido[]>([]);

  useEffect(() => {
    const conexao = abrirEventos({
      url: api.urlDeEventos(),
      ...(criarFonteDeEventos === undefined ? {} : { criarFonte: criarFonteDeEventos }),
      aoMudarEstado: setEstado,
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

  return <Contexto.Provider value={{ estado, recebidos }}>{children}</Contexto.Provider>;
}
