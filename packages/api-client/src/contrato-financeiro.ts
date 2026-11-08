/** Financeiro essencial e repasse (migration 0042). Conferido em `conformidade.ts`. */
export interface Titulo {
  id: string;
  codigo: string;
  tipo: 'pagar' | 'receber';
  descricao: string;
  contraparte: string;
  categoria: string;
  valor_centavos: number;
  vencimento: string;
  situacao: 'aberto' | 'pago' | 'cancelado';
  aprovacoes_exigidas: number;
  aprovacoes: number;
  vencido: boolean;
}
export interface NovoTitulo {
  tipo: 'pagar' | 'receber';
  descricao: string;
  contraparte: string;
  categoria: string;
  valor_centavos: number;
  vencimento: string;
}
export interface ResumoFinanceiro {
  entradas_do_mes_centavos: number;
  saidas_do_mes_centavos: number;
  saldo_em_contas_centavos: number;
  a_pagar_aberto_centavos: number;
  a_receber_aberto_centavos: number;
  aguardando_aprovacao: number;
  extrato_sem_conciliacao: number;
}
export interface RepasseDoMedico {
  medico_id: string;
  medico: string;
  laudos: number;
  base_centavos: number;
  percentual_pb?: number | null;
  regra_versao?: number | null;
  valor_centavos?: number | null;
  sem_preco: number;
}
export interface RepasseGerado {
  titulos_criados: number;
  ja_existiam: number;
  sem_regra: string[];
}
export interface ContaBancaria {
  id: string;
  codigo: string;
  nome: string;
}
export interface NotaFiscal {
  numero: string;
  codigo_verificacao: string;
  valor_centavos: number;
  situacao: 'emitida' | 'cancelada';
  emitida_em: string;
  cancelada_em?: string | null;
}
