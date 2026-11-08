/** Orçamento do particular (Orcamento.dc.html). O servidor calcula tudo (G1). Conferido em `conformidade.ts`. */

export type MotivoDeDesconto = 'paciente_frequente' | 'funcionario_ou_familiar' | 'condicao_social' | 'cobrir_outra_clinica';
export type PagamentoPrevisto = 'pix' | 'dinheiro' | 'cartao';

export interface ExameComPreco {
  tuss: string;
  nome: string;
  valor_centavos: number;
}

export interface PacoteDoCatalogo {
  codigo: string;
  nome: string;
  regra_versao: number;
  tuss: string[];
  valor_centavos: number;
}

export interface CatalogoDoOrcamento {
  tabela_versao: number;
  exames: ExameComPreco[];
  pacotes: PacoteDoCatalogo[];
  limite_desconto_pontos_base: number;
}

export interface NovoOrcamento {
  paciente_id: string;
  itens: string[];
  pacote?: string | null;
  desconto?: { pontos_base: number; motivo: MotivoDeDesconto } | null;
  pagamento_previsto: PagamentoPrevisto;
}

export interface ItemDoOrcamento {
  tuss: string;
  descricao: string;
  valor_centavos: number;
}

export interface Orcamento {
  id: string;
  codigo: string;
  paciente_id: string;
  paciente_nome: string;
  paciente_nascimento: string;
  itens: ItemDoOrcamento[];
  tabela_versao: number;
  pacote?: string | null;
  pacote_regra_versao?: number | null;
  subtotal_centavos: number;
  pacote_centavos: number;
  desconto_centavos: number;
  desconto_pontos_base?: number | null;
  desconto_motivo?: string | null;
  total_centavos: number;
  pagamento_previsto: string;
  valido_ate: string;
  situacao: 'emitido' | 'vencido' | 'cancelado';
  motivo_cancelamento?: string | null;
  criado_em: string;
}
