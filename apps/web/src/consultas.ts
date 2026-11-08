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
  filaDaSala: (salaId: string) => ['fila-da-sala', salaId] as const,
  horariosLivres: (salaId: string, data: string) => ['horarios-livres', salaId, data] as const,
  pacientes: (texto: string) => ['pacientes', texto] as const,
  filaDeLaudos: ['laudos', 'fila'] as const,
  laudo: (id: string) => ['laudos', id] as const,
  portalLaudos: ['portal', 'laudos'] as const,
  portalLaudo: (id: string) => ['portal', 'laudos', id] as const,
  link: (token: string) => ['link', token] as const,
  meuCaixa: ['caixa', 'meu'] as const,
  contingenciaAberta: ['contingencia', 'aberta'] as const,
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

export const consultaFilaDaSala = (api: ClienteVidaCor, salaId: string) =>
  queryOptions({ queryKey: chaves.filaDaSala(salaId), queryFn: () => api.salas.fila(salaId), refetchInterval: 30_000 });

export const consultaHorariosLivres = (api: ClienteVidaCor, salaId: string, data: string) =>
  queryOptions({ queryKey: chaves.horariosLivres(salaId, data), queryFn: () => api.salas.horariosLivres(salaId, data) });

export const consultaPacientes = (api: ClienteVidaCor, texto: string) =>
  queryOptions({ queryKey: chaves.pacientes(texto), queryFn: () => api.pacientes.buscar(texto), staleTime: 30_000 });

export const consultaFilaDeLaudos = (api: ClienteVidaCor) =>
  queryOptions({ queryKey: chaves.filaDeLaudos, queryFn: () => api.laudos.fila(), refetchInterval: 60_000 });

export const consultaLaudo = (api: ClienteVidaCor, id: string) =>
  queryOptions({ queryKey: chaves.laudo(id), queryFn: () => api.laudos.obter(id) });

export const consultaPortalLaudos = (api: ClienteVidaCor) =>
  queryOptions({ queryKey: chaves.portalLaudos, queryFn: () => api.portal.laudos() });

export const consultaPortalLaudo = (api: ClienteVidaCor, id: string) =>
  queryOptions({ queryKey: chaves.portalLaudo(id), queryFn: () => api.portal.laudo(id) });

export const consultaLink = (api: ClienteVidaCor, token: string) =>
  queryOptions({ queryKey: chaves.link(token), queryFn: () => api.portal.link(token) });

export const consultaMeuCaixa = (api: ClienteVidaCor) =>
  queryOptions({ queryKey: chaves.meuCaixa, queryFn: () => api.caixas.meu() });

export const consultaContingenciaAberta = (api: ClienteVidaCor) =>
  queryOptions({ queryKey: chaves.contingenciaAberta, queryFn: () => api.contingencias.aberta(), refetchInterval: 60_000 });
