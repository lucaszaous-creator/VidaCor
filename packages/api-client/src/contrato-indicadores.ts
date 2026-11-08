/** Indicadores agregados da gerência (sem paciente). Conferido em `conformidade.ts`. */
export interface ProducaoPorExame {
  exame: string;
  atendimentos: number;
}
export interface LaudoPorMedico {
  medico: string;
  laudos: number;
  horas_medias?: number | null;
}
export interface OcupacaoPorHora {
  dia_da_semana: number;
  hora: number;
  agendamentos: number;
}
export interface Indicadores {
  de: string;
  ate: string;
  agendamentos: number;
  atendimentos: number;
  faltas: number;
  taxa_de_faltas?: number | null;
  receita_centavos: number;
  recebimentos: number;
  ticket_medio_centavos?: number | null;
  tempo_medio_de_laudo_horas?: number | null;
  producao: ProducaoPorExame[];
  por_medico: LaudoPorMedico[];
  ocupacao: OcupacaoPorHora[];
}
