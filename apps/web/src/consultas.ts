import { queryOptions } from '@tanstack/react-query';
import type { ClienteVidaCor } from '@vidacor/api-client';

/** Chaves e consultas do TanStack Query. O cache vive só em memória (nada no navegador). */
export const chaves = {
  sessao: ['sessao'] as const,
  saude: ['saude'] as const,
  salas: ['salas'] as const,
  agendaTodas: ['agenda'] as const,
  agenda: (data: string, salaId: string | undefined) => ['agenda', data, salaId ?? 'todas'] as const,
  atendimento: (id: string) => ['atendimento', id] as const,
};

export const consultaSessao = (api: ClienteVidaCor) =>
  queryOptions({ queryKey: chaves.sessao, queryFn: () => api.sessao.obter(), staleTime: 60_000 });

export const consultaSaude = (api: ClienteVidaCor) =>
  queryOptions({ queryKey: chaves.saude, queryFn: () => api.saude(), retry: false, refetchInterval: 30_000 });

export const consultaSalas = (api: ClienteVidaCor) =>
  queryOptions({ queryKey: chaves.salas, queryFn: () => api.salas.listar(), staleTime: 5 * 60_000 });

export const consultaAgenda = (api: ClienteVidaCor, data: string, salaId: string | undefined) =>
  queryOptions({
    queryKey: chaves.agenda(data, salaId),
    queryFn: () => api.agenda.obter(salaId === undefined ? { data } : { data, sala_id: salaId }),
    refetchInterval: 60_000,
  });

export const consultaAtendimento = (api: ClienteVidaCor, id: string) =>
  queryOptions({ queryKey: chaves.atendimento(id), queryFn: () => api.atendimentos.obter(id) });
