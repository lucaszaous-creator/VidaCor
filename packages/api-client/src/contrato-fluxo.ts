/** Fluxo de laudos (Fluxo-Laudos.dc.html): só metadados, nunca o texto. Conferido em `conformidade.ts`. */

export type EtapaDoLaudo = 'a_laudar' | 'em_retificacao' | 'aguardando_conversa' | 'assinado' | 'entregue';

export interface CartaoDoFluxo {
  laudo_id: string;
  codigo: string;
  etapa: EtapaDoLaudo;
  paciente_nome: string;
  paciente_nascimento: string;
  exame: string;
  modalidade: string;
  medico?: string | null;
  recebido_em: string;
  prazo_em: string;
  assinado_em?: string | null;
  versao_vigente?: number | null;
  atrasado: boolean;
}

export interface FluxoDeLaudos {
  prazo_horas: number;
  etapas: { etapa: EtapaDoLaudo; quantidade: number }[];
  no_prazo_percentual?: number | null;
  atrasados: number;
  vencem_em_2h: number;
  cartoes: CartaoDoFluxo[];
  producao: { medico: string; assinados_hoje: number; tempo_medio_horas?: number | null; na_fila: number }[];
  retificacoes: { laudo_id: string; codigo: string; paciente_nome: string; exame: string; versao: number; assinada_em?: string | null; motivo?: string | null; medico?: string | null }[];
}
