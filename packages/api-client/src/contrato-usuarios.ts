/** Usuários e perfis (migration 0043). Conferido em `conformidade.ts`. */
export interface PedidoPendente {
  id: string;
  tipo: string;
  perfil?: string | null;
  motivo: string;
  pedido_por: string;
}
export interface UsuarioAdministrado {
  id: string;
  codigo: string;
  login: string;
  nome: string;
  perfis: string[];
  ativo: boolean;
  ultimo_acesso?: string | null;
  pendente?: PedidoPendente | null;
}
export interface NovoUsuario {
  login: string;
  nome: string;
  perfis: string[];
  motivo: string;
}
export interface MudancaDePerfil {
  tipo: 'conceder' | 'retirar' | 'desativar' | 'reativar';
  perfil?: string | null;
  motivo: string;
}
export interface DecisaoDePerfil {
  aprovar: boolean;
  motivo: string;
}

/* Exame externo (migration 0045). */
export interface EstudoExterno {
  study_uid: string;
  descricao: string;
  modalidade: string;
  data_estudo: string;
  imagens: number;
  sha256: string;
}
export interface ImportacaoExterna {
  instituicao: string;
  origem: 'link' | 'arquivo' | 'dicomweb';
  nome_na_origem: string;
  nascimento_na_origem: string;
  id_na_origem?: string | null;
  documento_conferido: boolean;
  estudos: EstudoExterno[];
}
export interface ExameExterno {
  id: string;
  codigo: string;
  instituicao: string;
  origem: string;
  nome_na_origem: string;
  id_na_origem?: string | null;
  criado_em: string;
  estudos: EstudoExterno[];
  somente_leitura: boolean;
}
