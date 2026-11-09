/** Qualidade (Qualidade.dc.html): não conformidades e eventos de segurança. Conferido em `conformidade.ts`. */

export type ClassificacaoDaNc = 'processo' | 'equipamento' | 'documentacao' | 'fornecedor';
export type GravidadeDaNc = 'leve' | 'moderada' | 'grave';
export type TipoDeEvento = 'reacao_contraste' | 'queda' | 'erro_identificacao' | 'exame_errado' | 'falha_equipamento' | 'outro';
export type GravidadeDoEvento = 'sem_dano' | 'leve' | 'moderado' | 'grave' | 'obito';
export type SituacaoDaNc = 'aberta' | 'em_acao' | 'verificacao' | 'encerrada';

export interface NovaNaoConformidade {
  descricao: string;
  classificacao: ClassificacaoDaNc;
  gravidade: GravidadeDaNc;
}

export interface NovoEventoAdverso {
  tipo: TipoDeEvento;
  quase_erro: boolean;
  gravidade: GravidadeDoEvento;
  descricao: string;
  paciente_id?: string | null;
  anonimo?: boolean;
}

export interface ProtocoloDeQualidade {
  id: string;
  codigo: string;
  notificar_anvisa_ate?: string | null;
}

export interface NaoConformidade {
  id: string;
  codigo: string;
  descricao: string;
  classificacao: string;
  gravidade: string;
  responsavel?: string | null;
  prazo_em?: string | null;
  situacao: SituacaoDaNc;
  eficaz?: boolean | null;
  vencida: boolean;
  aberta_por: string;
  aberta_em: string;
  versao: number;
  andamentos: { texto: string; situacao_nova: string; por: string; em: string }[];
}

export interface EventoDeSeguranca {
  id: string;
  codigo: string;
  tipo: string;
  quase_erro: boolean;
  gravidade: string;
  descricao: string;
  paciente?: string | null;
  notificado_por?: string | null;
  notificado_em: string;
  notificar_anvisa_ate?: string | null;
  situacao: 'recebido' | 'em_analise' | 'concluido';
}

export interface PainelDaQualidade {
  nc_abertas: number;
  nc_vencidas: number;
  eventos_no_mes: number;
  quase_erros_no_mes: number;
  graves_no_mes: number;
  anvisa_pendentes: number;
  nao_conformidades: NaoConformidade[];
  eventos: EventoDeSeguranca[];
}

export interface AndamentoNovo {
  texto: string;
  situacao: SituacaoDaNc;
  responsavel_id?: string | null;
  prazo_em?: string | null;
  eficaz?: boolean | null;
  versao: number;
}
