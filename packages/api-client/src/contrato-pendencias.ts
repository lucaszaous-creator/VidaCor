/**
 * Pendências da gerência no caixa (docs/30 § 7, migration 0017): pedido de desconto acima do limite
 * numa fila e estornos pedidos, com recusa motivada. Conferido contra o OpenAPI em `conformidade.ts`.
 */
import type { PacienteResumo } from './contrato-m0';

export interface Pessoa {
  id: string;
  nome: string;
}

export type SituacaoDoPedidoDeDesconto = 'pedido' | 'autorizado' | 'recusado' | 'cancelado';

export interface NovoPedidoDesconto {
  paciente_id: string;
  pontos_base: number;
  motivo: string;
}

export interface PedidoDesconto {
  id: string;
  codigo: string;
  paciente: PacienteResumo;
  pontos_base: number;
  motivo: string;
  solicitante: Pessoa;
  situacao: SituacaoDoPedidoDeDesconto;
  /** Preenchida quando autorizado; o recebimento usa este ID. */
  autorizacao_id?: string | null;
  decidido_por?: Pessoa | null;
  motivo_decisao?: string | null;
  criado_em: string;
}

export type SituacaoDoEstorno = 'pedido' | 'aprovado' | 'recusado';

export interface EstornoDetalhado {
  id: string;
  codigo: string;
  situacao: SituacaoDoEstorno;
  motivo: string;
  recebimento_id: string;
  recebimento_codigo: string;
  paciente: PacienteResumo;
  total_centavos: number;
  forma: string;
  pago_em?: string | null;
  pedido_por: Pessoa;
  pedido_em: string;
  decidido_por?: Pessoa | null;
  motivo_recusa?: string | null;
}

export interface Decisao {
  motivo: string;
}

/** O que o caixa tem a receber no dia (sem o exame da agenda: N3; procedimentos pela tabela, N2). */
export interface ProcedimentoACobrar {
  tuss: string;
  descricao: string;
  valor_centavos?: number | null;
}

export interface AtendimentoAReceber {
  atendimento_id: string;
  codigo: string;
  paciente: PacienteResumo;
  inicio: string;
  procedimentos: ProcedimentoACobrar[];
  recebido: boolean;
}
