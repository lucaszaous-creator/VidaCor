/** Convite e login do portal do paciente (migration 0019). Conferido em `conformidade.ts`. */

export interface ConviteEmitido {
  id: string;
  codigo: string;
  expira_em: string;
  /** E-mail do cadastro, mascarado: o link vai só para lá (a recepção não o vê). */
  enviado_para: string;
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

/* Cadastro de paciente (migration 0020). */
export interface PossivelDuplicado {
  paciente: { id: string; nome: string; nascimento: string };
  codigo: string;
  motivo: string;
  /** Mesmo CPF: não pode ser descartado. */
  bloqueante: boolean;
}

export interface NovoPaciente {
  nome: string;
  nome_social?: string | null;
  nascimento: string;
  sexo: 'F' | 'M';
  cpf?: string | null;
  nome_mae?: string | null;
  celular: string;
  email?: string | null;
  alergias: string[];
  nega_alergias: boolean;
  duplicado_conferido: boolean;
}

export interface PacienteCadastrado {
  id: string;
  codigo: string;
  nome: string;
  nascimento: string;
}
