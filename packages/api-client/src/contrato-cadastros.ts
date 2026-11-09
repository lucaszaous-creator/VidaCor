/**
 * Cadastros base: Corpo clínico (Corpo-Clinico.dc.html), Exames e procedimentos com a tabela de preços
 * versionada (Exames-Procedimentos.dc.html) e o cabeçalho impresso nos documentos da unidade.
 * Conferido em `conformidade.ts`.
 */

export interface DadosDoRegistro {
  uf: string;
  numero: string;
  rqe?: string | null;
  especialidade?: string | null;
}

export interface DadosDaHabilitacao {
  realiza: string[];
  lauda: string[];
}

export interface RegistroVigente {
  conselho: string;
  uf: string;
  numero: string;
  rqe?: string | null;
  especialidade?: string | null;
  desde: string;
}

export interface HabilitacaoVigente {
  realiza: string[];
  lauda: string[];
  desde: string;
}

export interface PedidoDoCorpoClinico {
  id: string;
  codigo: string;
  tipo: 'registro' | 'habilitacao';
  situacao: 'aguardando' | 'aplicado' | 'recusado';
  registro?: DadosDoRegistro | null;
  habilitacao?: DadosDaHabilitacao | null;
  motivo: string;
  pedido_por: string;
  pedido_por_id: string;
  pedido_em: string;
  decidido_por?: string | null;
  decidido_em?: string | null;
  motivo_decisao?: string | null;
}

export interface MedicoDoCorpoClinico {
  id: string;
  codigo: string;
  nome: string;
  ativo: boolean;
  situacao: 'ativo' | 'aguardando_conferencia' | 'sem_registro' | 'inativo';
  registro?: RegistroVigente | null;
  habilitacao?: HabilitacaoVigente | null;
  pendentes: PedidoDoCorpoClinico[];
  /** Última consulta do CRM no CFM. */
  consulta_crm?: ConsultaCrm | null;
  /** Certificado ICP-Brasil vigente. */
  certificado?: CertificadoDoMedico | null;
}

export type SituacaoNoConselho = 'regular' | 'suspenso' | 'cassado' | 'cancelado' | 'falecido' | 'transferido' | 'interdicao_cautelar' | 'nao_encontrado';

export interface ConsultaCrm {
  id: string;
  codigo: string;
  uf: string;
  numero: string;
  fonte: 'webservice_cfm' | 'portal_cfm';
  situacao: SituacaoNoConselho;
  nome_no_conselho?: string | null;
  especialidades: string[];
  resposta_sha256?: string | null;
  observacao?: string | null;
  consultado_em: string;
  consultado_por: string;
}

export interface NovaConsultaCrm {
  uf: string;
  numero: string;
  fonte: 'webservice_cfm' | 'portal_cfm';
  situacao?: SituacaoNoConselho | null;
  nome_no_conselho?: string | null;
  observacao?: string | null;
}

export interface CertificadoDoMedico {
  id: string;
  codigo: string;
  titular: string;
  cpf_mascarado?: string | null;
  emissor: string;
  serie: string;
  tipo?: 'A1' | 'A2' | 'A3' | 'A4' | 'outro' | null;
  valido_de: string;
  valido_ate: string;
  sha256: string;
  /** Dias até vencer (negativo: vencido), calculado pelo servidor. */
  vence_em_dias: number;
  cadastrado_em: string;
  cadastrado_por: string;
}

export interface NovoCertificado {
  /** Arquivo .cer (PEM ou DER) em base64; só o certificado público. */
  arquivo_base64: string;
  motivo?: string | null;
}

export interface NovoPedidoDoCorpoClinico {
  tipo: 'registro' | 'habilitacao';
  registro?: DadosDoRegistro | null;
  habilitacao?: DadosDaHabilitacao | null;
  motivo: string;
}

export interface DecisaoDoCorpoClinico {
  aprovar: boolean;
  motivo: string;
}

export interface ProcedimentoDoCatalogo {
  tuss: string;
  nome: string;
  modalidade?: string | null;
  duracao_min: number;
  requer_contraste: boolean;
  ativo: boolean;
  versao: number;
  preco_centavos?: number | null;
  proximo_preco_centavos?: number | null;
}

export interface VersaoDaTabelaDePreco {
  id: string;
  versao: number;
  vigente_desde: string;
  descricao: string;
  motivo?: string | null;
  itens: number;
  publicada_por?: string | null;
  publicada_em: string;
  situacao: 'vigente' | 'futura' | 'anterior';
}

export interface CatalogoDeExames {
  procedimentos: ProcedimentoDoCatalogo[];
  tabelas: VersaoDaTabelaDePreco[];
}

export interface EstadoDoProcedimento {
  nome: string;
  modalidade?: string | null;
  duracao_min: number;
  requer_contraste: boolean;
  ativo: boolean;
}

export interface MudancaRegistradaNoCatalogo {
  codigo: string;
  tipo: 'incluir' | 'alterar' | 'desativar' | 'reativar';
  motivo: string;
  versao_do_procedimento: number;
  antes?: EstadoDoProcedimento | null;
  depois: EstadoDoProcedimento;
  quem: string;
  em: string;
}

export interface NovoProcedimento {
  tuss: string;
  nome: string;
  modalidade: string;
  duracao_min: number;
  requer_contraste: boolean;
  motivo: string;
}

export interface MudancaNoProcedimento {
  versao: number;
  duracao_min?: number | null;
  modalidade?: string | null;
  ativo?: boolean | null;
  motivo: string;
}

export interface PrecoDaVersao {
  tuss: string;
  valor_centavos: number;
}

export interface NovaVersaoDaTabela {
  vigente_desde: string;
  descricao: string;
  motivo: string;
  precos: PrecoDaVersao[];
}

export interface CabecalhoDaUnidade {
  numero: number;
  nome: string;
  razao_social?: string | null;
  cnpj?: string | null;
  endereco?: string | null;
  telefone?: string | null;
  site?: string | null;
  responsavel_nome?: string | null;
  responsavel_crm?: string | null;
  responsavel_tecnico?: string | null;
  motivo?: string | null;
  publicado_por?: string | null;
  desde: string;
}

export interface CabecalhoDosDocumentos {
  vigente?: CabecalhoDaUnidade | null;
  versoes: CabecalhoDaUnidade[];
}

export interface NovoCabecalho {
  numero_atual?: number | null;
  nome: string;
  razao_social: string;
  cnpj: string;
  endereco: string;
  telefone: string;
  site?: string | null;
  responsavel_nome: string;
  responsavel_crm_uf: string;
  responsavel_crm_numero: string;
  motivo?: string | null;
}
