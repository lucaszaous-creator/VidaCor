/** Convite e login do portal do paciente (migration 0019). Conferido em `conformidade.ts`. */

export interface ConviteEmitido {
  id: string;
  codigo: string;
  /** Link do aceite; só na primeira resposta (na repetição vem nulo). */
  caminho?: string | null;
  expira_em: string;
}

export interface SituacaoDoConvite {
  situacao: 'valido' | 'indisponivel';
}

export interface AceiteDoConvite {
  nascimento: string;
  login: string;
  senha: string;
}

export interface EntradaNoPortal {
  login: string;
  senha: string;
}

export interface SessaoDoPortal {
  primeiro_nome: string;
}
