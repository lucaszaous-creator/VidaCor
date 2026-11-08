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
