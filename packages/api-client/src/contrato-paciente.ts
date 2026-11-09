/** Paciente 360 (Paciente.dc.html): visão geral. Conferido em `conformidade.ts`. */

export interface IdentificacaoDoPaciente {
  id: string;
  codigo: string;
  nome: string;
  nome_social?: string | null;
  nascimento: string;
  sexo?: string | null;
  cpf_mascarado?: string | null;
  celular?: string | null;
  email?: string | null;
  alergias: string[];
  nega_alergias?: boolean | null;
}

export interface AgendamentoDoPaciente {
  id: string;
  codigo: string;
  inicio: string;
  exame: string;
  sala: string;
  medico?: string | null;
  situacao: string;
}

export interface VisaoDoPaciente {
  identificacao: IdentificacaoDoPaciente;
  proximos: AgendamentoDoPaciente[];
  historico: AgendamentoDoPaciente[];
  faltas: number;
  exames_externos: number;
}

/** Lista de pacientes (Pacientes-Lista.dc.html): página, recortes e situação calculados pelo núcleo. */
export type RecorteDaLista = 'todos' | 'hoje' | 'pendencia' | 'incompleto';
export type SituacaoNaLista = 'na_clinica' | 'pendencia' | 'cadastro_incompleto' | 'possivel_duplicado' | 'em_dia';

export interface ExameDaLista {
  exame: string;
  inicio: string;
}

export interface LinhaDaListaDePacientes {
  id: string;
  codigo: string;
  nome: string;
  nascimento: string;
  convenio: string;
  cpf_mascarado?: string | null;
  celular?: string | null;
  alergias: string[];
  ultimo_exame?: ExameDaLista | null;
  proximo_exame?: ExameDaLista | null;
  situacao: SituacaoNaLista;
  falta_no_cadastro: string[];
}

export interface ListaDePacientes {
  cadastrados: number;
  recortes: { todos: number; hoje: number; pendencia: number; incompleto: number; duplicados: number };
  pagina: number;
  paginas: number;
  por_pagina: number;
  total: number;
  itens: LinhaDaListaDePacientes[];
}

export interface CadastroDoPar {
  id: string;
  codigo: string;
  nome: string;
  nascimento: string;
  cpf_mascarado?: string | null;
  celular?: string | null;
}

export interface ParDuplicado {
  a: CadastroDoPar;
  b: CadastroDoPar;
  motivo: string;
  situacao: 'aberto' | 'juncao_pedida';
  decidido_em?: string | null;
  decidido_por?: string | null;
}

export interface DecisaoSobreDuplicado {
  paciente_a: string;
  paciente_b: string;
  decisao: 'juncao_pedida' | 'pessoas_diferentes';
  motivo?: string | null;
}

export interface DecisaoRegistrada {
  id: string;
  codigo: string;
  decisao: string;
}
