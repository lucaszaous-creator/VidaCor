/** Lista de espera, ofertas de vaga, bloqueios e cancelamento (Lista-Espera.dc.html). Conferido em `conformidade.ts`. */

export type PrioridadeDeEspera = 'urgente' | 'oncologico' | 'gestante' | 'rotina';
export type PreferenciaDeHorario = 'qualquer' | 'manha' | 'tarde';
export type TipoDeBloqueio = 'feriado' | 'manutencao' | 'outro';

interface PacienteDaEspera {
  id: string;
  nome: string;
  nascimento: string;
}

export interface OfertaDoPedido {
  id: string;
  codigo: string;
  sala_nome: string;
  inicio: string;
  expira_em: string;
}

export interface PedidoDeEspera {
  id: string;
  codigo: string;
  posicao?: number | null;
  paciente: PacienteDaEspera;
  exame: { codigo_tuss: string; nome: string };
  prioridade: string;
  preferencia: string;
  justificativa?: string | null;
  medico_solicitante?: string | null;
  operadora?: string | null;
  guia_codigo?: string | null;
  situacao: string;
  desde: string;
  oferta?: OfertaDoPedido | null;
}

export interface PedidoInclusao {
  paciente_id: string;
  exame_tuss: string;
  prioridade: PrioridadeDeEspera;
  preferencia: PreferenciaDeHorario;
  justificativa?: string | null;
  medico_solicitante?: string | null;
  operadora?: string | null;
  guia_id?: string | null;
}

export interface OfertaDeVaga {
  id: string;
  codigo: string;
  espera_id: string;
  espera_codigo: string;
  paciente: PacienteDaEspera;
  exame: { codigo_tuss: string; nome: string };
  sala: { id: string; nome: string };
  inicio: string;
  fim: string;
  expira_em: string;
  situacao: string;
  origem_vaga: string;
  agendamento_codigo?: string | null;
  motivo?: string | null;
}

export interface PedidoBloqueio {
  sala_ids: string[];
  tipo: TipoDeBloqueio;
  inicio: string;
  fim: string;
  motivo: string;
}

export interface BloqueioDeSala {
  id: string;
  codigo: string;
  sala: { id: string; nome: string };
  tipo: string;
  inicio: string;
  fim: string;
  motivo: string;
  situacao: string;
  retirado_em?: string | null;
  motivo_retirada?: string | null;
}
