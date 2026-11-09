/**
 * Preparo do exame (Preparo-Exame.dc.html): passos que o paciente recebe no comprovante do
 * agendamento, versionados por procedimento. Conferido em `conformidade.ts`.
 */

export interface VersaoDoPreparo {
  id: string;
  codigo: string;
  numero: number;
  passos: string[];
  chegar_antes_min: number;
  vigente: boolean;
  motivo?: string | null;
  criado_em: string;
  criado_por: string;
}

export interface PreparoDoExame {
  tuss: string;
  nome: string;
  modalidade?: string | null;
  /** Ausente quando a clínica ainda não cadastrou orientação para o exame. */
  vigente?: VersaoDoPreparo | null;
}

export interface NovoPreparo {
  /** Número da versão vigente que a pessoa viu (ausente quando ainda não há nenhuma). */
  versao_vista?: number | null;
  passos: string[];
  chegar_antes_min: number;
  /** Obrigatório a partir da segunda versão. */
  motivo?: string | null;
}
