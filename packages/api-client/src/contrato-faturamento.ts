/**
 * Faturamento TISS com auditoria antes do lote (migration 0050, DEC-2026-11-08-65). Conferido com o
 * contrato gerado em `conformidade.ts`. Valores sempre em centavos; a tela não calcula nada (G1).
 */

export type SituacaoDaConta =
  | 'em_conferencia'
  | 'pronta'
  | 'com_pendencia'
  | 'em_lote'
  | 'enviada'
  | 'paga'
  | 'paga_com_glosa'
  | 'glosada';

export type EfeitoDaPendencia = 'bloqueia' | 'corrigir_antes' | 'atencao';
export type SituacaoDoLote = 'gerado' | 'aguardando_envio' | 'enviado' | 'demonstrativo_recebido';
export type SituacaoDaGlosa = 'aberta' | 'em_recurso';

export interface PacienteDaConta {
  nome: string;
  nascimento: string;
}

export interface ContaResumo {
  id: string;
  /** Número da guia no prestador (FAT-AAAA-NNNNNN). */
  codigo: string;
  paciente: PacienteDaConta;
  operadora: string;
  numero_guia_operadora: string;
  procedimentos: string[];
  valor_total_centavos: number;
  situacao: SituacaoDaConta;
  data_execucao: string;
  enviar_ate?: string | null;
  pendencias_que_impedem: number;
  pendencias_de_atencao: number;
  regra_versao?: string | null;
  lote_codigo?: string | null;
}

export interface PendenciaDaConta {
  regra: string;
  regra_versao: string;
  efeito: EfeitoDaPendencia;
  origem: string;
  titulo: string;
  detalhe: string;
  o_que_fazer: string;
}

export interface ItemDaConta {
  sequencial: number;
  tuss: string;
  descricao: string;
  quantidade: number;
  valor_unitario_centavos?: number | null;
}

export interface Solicitante {
  nome: string;
  conselho?: string | null;
  numero?: string | null;
  uf?: string | null;
  cbos?: string | null;
}

export interface VersaoDaConta {
  versao: number;
  motivo: string;
  criado_em: string;
}

export interface RecursoDeGlosa {
  id: string;
  codigo: string;
  justificativa: string;
  situacao: 'aguardando_envio' | 'enviado';
  criado_em: string;
}

export interface Glosa {
  id: string;
  codigo: string;
  conta_id: string;
  conta_codigo: string;
  paciente: PacienteDaConta;
  operadora: string;
  codigo_glosa: string;
  motivo: string;
  valor_centavos: number;
  prazo_recurso: string;
  situacao: SituacaoDaGlosa;
  recurso?: RecursoDeGlosa | null;
}

export interface ContaDetalhe {
  resumo: ContaResumo;
  carteirinha: string;
  senha: string;
  validade_senha: string;
  solicitante: Solicitante;
  cid?: string | null;
  assinatura_beneficiario: boolean;
  itens: ItemDaConta[];
  pendencias: PendenciaDaConta[];
  historico: VersaoDaConta[];
  versao: number;
  glosas: Glosa[];
}

export interface ResultadoConferencia {
  novas: number;
  conferidas: number;
  prontas: number;
  com_pendencia: number;
}

export interface ResumoAuditoria {
  conferidas: number;
  prontas: number;
  corrigir_antes: number;
  bloqueadas: number;
}

export interface ContaPendente {
  conta_id: string;
  codigo: string;
  paciente: PacienteDaConta;
  operadora: string;
  numero_guia_operadora: string;
  detalhe: string;
}

export interface GrupoDePendencias {
  regra: string;
  titulo: string;
  efeito: EfeitoDaPendencia;
  origem: string;
  o_que_fazer: string;
  regra_versoes: string[];
  contas: ContaPendente[];
}

export interface RegraDoConjunto {
  regra: string;
  titulo: string;
  efeito: EfeitoDaPendencia;
  origem: string;
}

export interface ConjuntoDeRegras {
  versao: string;
  nome: string;
  operadoras: string[];
  regras: RegraDoConjunto[];
}

export interface PainelAuditoria {
  resumo: ResumoAuditoria;
  grupos: GrupoDePendencias[];
  conjuntos: ConjuntoDeRegras[];
}

export interface ContratoDeConvenio {
  id: string;
  codigo: string;
  operadora: string;
  registro_ans: string;
  regra_versao: string;
  prazo_envio_dias: number;
  prazo_recurso_dias: number;
  contas_prontas: number;
}

export interface EnvioPendente {
  tentativas: number;
  ultimo_erro?: string | null;
}

export interface LoteTiss {
  id: string;
  codigo: string;
  numero: number;
  contrato_id: string;
  operadora: string;
  quantidade_guias: number;
  valor_total_centavos: number;
  situacao: SituacaoDoLote;
  versao_tiss: string;
  hash_tiss: string;
  sha256: string;
  protocolo?: string | null;
  criado_em: string;
  enviado_em?: string | null;
  envio_pendente?: EnvioPendente | null;
  valor_pago_centavos?: number | null;
  valor_glosado_centavos?: number | null;
}

export interface LoteDetalhe {
  lote: LoteTiss;
  contas: ContaResumo[];
}

export interface PedidoLote {
  contrato_id: string;
}

export interface SolicitanteCorrigido {
  conselho: string;
  numero: string;
  uf: string;
  cbos: string;
}

export interface PedidoCorrecaoConta {
  motivo: string;
  solicitante?: SolicitanteCorrigido | null;
  cid?: string | null;
  assinatura_beneficiario?: boolean | null;
  reaplicar_tabela?: boolean;
}

export interface GlosaRecebida {
  codigo_glosa: string;
  motivo: string;
  valor_centavos: number;
}

export interface RetornoDaConta {
  conta_id: string;
  valor_pago_centavos: number;
  glosas?: GlosaRecebida[];
}

export interface PedidoDemonstrativo {
  recebido_em: string;
  contas: RetornoDaConta[];
}

export interface PedidoRecurso {
  justificativa: string;
}
