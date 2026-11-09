/**
 * Convênios e tabelas (Convenios-Tabelas.dc.html) e Check-up e pacotes (Checkup-Pacotes.dc.html).
 * Tipos escritos à mão que estreitam as situações; conferidos com o gerado em `conformidade.ts`.
 * Valores em centavos; quem calcula preço, reajuste, avulso e economia é o servidor (G1).
 */

export interface TermosDoConvenio {
  codigo_prestador: string;
  nome_prestador: string;
  cnes: string;
  regra_versao: string;
  prazo_envio_dias: number;
  prazo_recurso_dias: number;
  prazo_pagamento_dias?: number | null;
  vigencia_inicio?: string | null;
  vigencia_fim?: string | null;
  reajuste?: string | null;
  contato_nome?: string | null;
  contato_telefone?: string | null;
  contato_email?: string | null;
  autorizacao_por_webservice: boolean;
  ativo: boolean;
}

export type SituacaoDoConvenio = 'vigente' | 'renovar' | 'vencido' | 'sem_vigencia' | 'inativo';

export interface ConvenioResumo {
  id: string;
  codigo: string;
  operadora: string;
  registro_ans: string;
  situacao: SituacaoDoConvenio;
  vigencia_fim?: string | null;
  tabela_versao?: number | null;
  proposta_pendente: boolean;
}

export interface VersaoDaTabelaDoConvenio {
  id: string;
  codigo: string;
  versao: number;
  vigente_desde: string;
  descricao: string;
  motivo?: string | null;
  itens: number;
  proposta_por?: string | null;
  aprovada_por?: string | null;
  criada_em: string;
  situacao: 'vigente' | 'futura' | 'anterior';
}

export interface MudancaDePrecoDoConvenio {
  tuss: string;
  descricao: string;
  antes_centavos?: number | null;
  depois_centavos: number;
}

export interface PropostaDeTabela {
  id: string;
  codigo: string;
  contrato_id: string;
  tipo: 'reajuste' | 'precos';
  reajuste_pontos_base?: number | null;
  base_versao?: number | null;
  vigente_desde: string;
  descricao: string;
  documento: string;
  motivo: string;
  situacao: 'pendente' | 'aprovada' | 'recusada';
  proposta_por: string;
  proposta_por_id: string;
  proposta_em: string;
  decidido_por?: string | null;
  decidido_em?: string | null;
  motivo_decisao?: string | null;
  itens: number;
  total_antes_centavos: number;
  total_depois_centavos: number;
  maiores: MudancaDePrecoDoConvenio[];
  tabela_versao?: number | null;
}

export interface RegraDeAutorizacaoDoExame {
  tuss: string;
  nome: string;
  exige_senha: boolean;
  validade_dias?: number | null;
  documentos: string;
}

export interface RegrasDeAutorizacaoDoConvenio {
  codigo: string;
  versao_regras: number;
  motivo: string;
  publicada_por: string;
  publicada_em: string;
  itens: RegraDeAutorizacaoDoExame[];
}

export interface RegraDaAuditoriaDoConvenio {
  regra: string;
  titulo: string;
  efeito: 'bloqueia' | 'corrigir_antes' | 'atencao';
  origem: string;
}

export interface ConjuntoDeAuditoriaDisponivel {
  versao: string;
  nome: string;
}

export interface AuditoriaDoConvenio {
  versao: string;
  nome: string;
  regras: RegraDaAuditoriaDoConvenio[];
  disponiveis: ConjuntoDeAuditoriaDisponivel[];
}

export interface MudancaDoConvenio {
  codigo: string;
  tipo: 'cadastrar' | 'alterar' | 'desativar' | 'reativar';
  motivo: string;
  versao_do_contrato: number;
  antes?: TermosDoConvenio | null;
  depois: TermosDoConvenio;
  quem: string;
  em: string;
}

export interface ConvenioDetalhe {
  id: string;
  codigo: string;
  operadora: string;
  registro_ans: string;
  situacao: SituacaoDoConvenio;
  versao: number;
  termos: TermosDoConvenio;
  tabelas: VersaoDaTabelaDoConvenio[];
  proposta_pendente?: PropostaDeTabela | null;
  regras_autorizacao?: RegrasDeAutorizacaoDoConvenio | null;
  auditoria: AuditoriaDoConvenio;
  historico: MudancaDoConvenio[];
}

export interface NovoConvenio {
  operadora: string;
  registro_ans: string;
  termos: TermosDoConvenio;
  motivo: string;
}

export interface AlteracaoDoConvenio {
  versao: number;
  termos: TermosDoConvenio;
  motivo: string;
}

export interface PrecoDoConvenio {
  tuss: string;
  descricao: string;
  valor_centavos: number;
  valor_anterior_centavos?: number | null;
}

export interface PrecosDoConvenio {
  versao: number;
  vigente_desde: string;
  descricao: string;
  itens: PrecoDoConvenio[];
}

export interface PreviaDeReajuste {
  base_versao: number;
  pontos_base: number;
  afetados: number;
  total_antes_centavos: number;
  total_depois_centavos: number;
  maiores: MudancaDePrecoDoConvenio[];
}

export interface PrecoPropostoDoConvenio {
  tuss: string;
  valor_centavos: number;
}

export interface NovaPropostaDeTabela {
  vigente_desde: string;
  descricao: string;
  documento: string;
  motivo: string;
  reajuste_pontos_base?: number | null;
  precos?: PrecoPropostoDoConvenio[] | null;
}

export interface DecisaoDaPropostaDeTabela {
  aprovar: boolean;
  motivo?: string | null;
}

export interface RegraDeAutorizacaoPedida {
  tuss: string;
  exige_senha: boolean;
  validade_dias?: number | null;
  documentos: string;
}

export interface NovasRegrasDeAutorizacao {
  versao_vista?: number | null;
  motivo: string;
  itens: RegraDeAutorizacaoPedida[];
}

// Check-up e pacotes.

export interface ExameDoPacoteDeCheckup {
  ordem: number;
  tuss: string;
  nome: string;
  duracao_min: number;
  preparo?: string | null;
  porque?: string | null;
  avulso_centavos?: number | null;
}

export interface VersaoDoPacoteDeCheckup {
  id: string;
  regra_versao: number;
  nome: string;
  para_quem?: string | null;
  vigente_desde: string;
  vigente_ate?: string | null;
  preco_centavos: number;
  motivo?: string | null;
  publicada_por?: string | null;
  publicada_em: string;
  situacao: 'vigente' | 'futura' | 'anterior';
  avulso_centavos?: number | null;
  economia_centavos?: number | null;
  desconto_pontos_base?: number | null;
  vale_no_orcamento: boolean;
  duracao_total_min: number;
  exames: ExameDoPacoteDeCheckup[];
}

export interface PacoteDeCheckup {
  codigo: string;
  nome: string;
  versoes: VersaoDoPacoteDeCheckup[];
}

export interface CatalogoDePacotes {
  pacotes: PacoteDeCheckup[];
  desconto_maximo_pontos_base: number;
}

export interface ExamePedidoNoPacote {
  tuss: string;
  preparo: string;
  porque: string;
}

export interface NovoPacoteDeCheckup {
  nome: string;
  para_quem: string;
  vigente_desde: string;
  preco_centavos: number;
  motivo: string;
  exames: ExamePedidoNoPacote[];
}

export interface NovaVersaoDoPacoteDeCheckup extends NovoPacoteDeCheckup {
  regra_versao_vista: number;
}
