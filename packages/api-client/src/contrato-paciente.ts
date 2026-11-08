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
