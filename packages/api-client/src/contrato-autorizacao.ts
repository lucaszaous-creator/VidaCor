/**
 * Autorização pedida pelo sistema (migration 0060, DEC-2026-11-08-80). Conferido com o contrato
 * gerado em `conformidade.ts`. A tela pede e mostra; a guia é preenchida pelo servidor (G1).
 */

export type SituacaoDaAutorizacao =
  | 'aguardando_envio'
  | 'aguardando_portal'
  | 'enviada'
  | 'em_analise'
  | 'pendente_documento'
  | 'autorizada'
  | 'parcial'
  | 'negada'
  | 'cancelada'
  | 'recusada';

export type CanalDaAutorizacao = 'webservice' | 'portal';

export interface SolicitanteDoPedido {
  nome: string;
  conselho: string;
  numero: string;
  uf: string;
  cbos: string;
}

export interface NovoPedidoAutorizacao {
  agendamento_id: string;
  operadora: string;
  carteirinha: string;
  solicitante: SolicitanteDoPedido;
  indicacao_clinica?: string | null;
}

export interface PedidoProtocolo {
  protocolo: string;
}

export interface ProcedimentoRespondido {
  tuss: string;
  autorizado: number;
}

export interface RespostaDoPortal {
  situacao: 'autorizada' | 'parcial' | 'negada' | 'em_analise' | 'pendente_documento';
  numero_guia_operadora?: string | null;
  senha?: string | null;
  data_autorizacao?: string | null;
  validade?: string | null;
  procedimentos?: ProcedimentoRespondido[];
  motivo?: string | null;
}

export interface AutorizacaoResumo {
  id: string;
  codigo: string;
  paciente: { nome: string; nascimento: string };
  operadora: string;
  canal: CanalDaAutorizacao;
  situacao: SituacaoDaAutorizacao;
  procedimentos: string[];
  protocolo?: string | null;
  motivo?: string | null;
  agendamento_id: string;
  inicio_do_exame: string;
  criado_em: string;
}

export interface ProcedimentoDoPedido {
  tuss: string;
  descricao: string;
  solicitado: number;
  autorizado?: number | null;
}

export interface RespostaRecebida {
  origem: CanalDaAutorizacao;
  sha256: string;
  situacao_lida?: string | null;
  erro_leitura?: string | null;
  hash_confere?: boolean | null;
  aplicada: boolean;
  recebida_em: string;
}

export interface GuiaPreenchida {
  id: string;
  codigo: string;
  numero: string;
  senha: string;
  validade: string;
  situacao: string;
}

export interface ConsultaPendente {
  tentativas: number;
  proxima_em: string;
  ultimo_erro?: string | null;
}

export interface AutorizacaoDetalhe {
  resumo: AutorizacaoResumo;
  carteirinha: string;
  solicitante: string;
  indicacao_clinica?: string | null;
  enviada_em?: string | null;
  respondida_em?: string | null;
  itens: ProcedimentoDoPedido[];
  respostas: RespostaRecebida[];
  guia?: GuiaPreenchida | null;
  pendente_no_outbox?: ConsultaPendente | null;
  situacao_do_agendamento: string;
}
