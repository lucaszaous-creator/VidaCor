/**
 * Minha agenda do médico, gestão de dose e relatórios (Agente 2, DEC-2026-11-09-140 a 154).
 * Tipos escritos à mão que estreitam situações para listas fechadas; conferidos com o gerado em
 * `conformidade-gestao.ts`.
 */
import type { PacienteResumo } from './contrato-m0';

// ---------------------------------------------------------------- Minha agenda
export type GrupoDaMinhaAgenda = 'a_chegar' | 'esperando' | 'em_exame' | 'liberado' | 'faltou';

export interface ItemDaMinhaAgenda {
  agendamento_id: string;
  codigo: string;
  inicio: string;
  paciente: PacienteResumo;
  exame: string;
  sala: string;
  grupo: GrupoDaMinhaAgenda;
  atendimento_id?: string | null;
  esperando_desde?: string | null;
  senha?: string | null;
  laudo_situacao?: string | null;
  /** Presente só quando o médico pode abrir o laudo (vínculo de cuidado). */
  laudo_id?: string | null;
}

export interface AvisoParaOMedico {
  tipo: 'imagens_recebidas' | 'laudo_vence';
  texto: string;
  quando: string;
}

export interface MinhaAgenda {
  data: string;
  medico: string;
  turnos: { sala: string; inicio: string; fim: string }[];
  pacientes: number;
  ja_atendidos: number;
  atendidos_com_laudo_pendente: number;
  esperando: number;
  laudos_para_assinar: number;
  laudos_vencem_hoje: number;
  itens: ItemDaMinhaAgenda[];
  avisos: AvisoParaOMedico[];
  semana: { data: string; pacientes: number; ausencia?: string | null }[];
}

export type MotivoDaAusencia = 'congresso' | 'ferias' | 'licenca' | 'pessoal';
export type SituacaoDaAusencia = 'aguardando' | 'aprovada' | 'recusada' | 'cancelada';

export interface AusenciaDoMedico {
  id: string;
  codigo: string;
  medico_id: string;
  medico: string;
  inicio: string;
  fim: string;
  motivo: MotivoDaAusencia;
  detalhe?: string | null;
  afetados_no_pedido: number;
  situacao: SituacaoDaAusencia;
  decidido_por?: string | null;
  decidido_em?: string | null;
  motivo_decisao?: string | null;
  afetados_na_decisao?: number | null;
  criado_em: string;
}

export interface NovaAusencia {
  inicio: string;
  fim: string;
  motivo: MotivoDaAusencia;
  detalhe?: string | null;
}

export interface DecisaoDaAusencia {
  aprovar: boolean;
  motivo?: string | null;
}

// ---------------------------------------------------------------- Gestão de dose
export type GrandezaDeDose = 'ctdivol' | 'dlp' | 'dap';
export type MotivoDeJustificativa = 'imc_alto' | 'repeticao_movimento' | 'fase_extra' | 'implante_metalico' | 'outro';

export interface JustificativaDeDose {
  motivo: MotivoDeJustificativa;
  detalhe?: string | null;
  por: string;
  em: string;
}

export interface ExameComDose {
  dose_id: string;
  codigo: string;
  paciente: PacienteResumo;
  menor_de_idade: boolean;
  exame: string;
  modalidade: string;
  aparelho?: string | null;
  realizado_em: string;
  ctdivol_mgy?: number | null;
  dlp_mgycm?: number | null;
  dap_gycm2?: number | null;
  nrd?: { grandeza: GrandezaDeDose; valor: number } | null;
  valor_comparado?: number | null;
  acima_do_nrd: boolean;
  justificativa?: JustificativaDeDose | null;
}

export interface ExamesComDose {
  de: string;
  ate: string;
  exames: ExameComDose[];
  acima_do_nrd: number;
  sem_justificativa: number;
}

export interface HistoricoDeDose {
  paciente: PacienteResumo;
  exames: {
    exame: string;
    aparelho?: string | null;
    realizado_em: string;
    dlp_mgycm?: number | null;
    dap_gycm2?: number | null;
    dose_efetiva_msv?: number | null;
  }[];
  dose_efetiva_12_meses_msv?: number | null;
  exames_sem_estimativa: number;
}

export interface ProtocoloDeDose {
  exame_tuss: string;
  exame: string;
  aparelho: string;
  grandeza: string;
  exames: number;
  mediana?: number | null;
  nrd?: number | null;
  percentual_do_nrd?: number | null;
  acima: number;
}

export interface NovaJustificativa {
  motivo: MotivoDeJustificativa;
  detalhe?: string | null;
}

export interface NrdDoExame {
  tuss: string;
  exame: string;
  modalidade?: string | null;
  numero?: number | null;
  grandeza?: GrandezaDeDose | null;
  valor?: number | null;
  fator_dose_efetiva?: number | null;
  motivo?: string | null;
  publicado_em?: string | null;
}

export interface NovoNrd {
  versao_vista?: number | null;
  grandeza: GrandezaDeDose;
  valor: number;
  fator_dose_efetiva?: number | null;
  motivo?: string | null;
}

export interface PeriodoDeConsulta {
  de?: string;
  ate?: string;
}

// ---------------------------------------------------------------- Relatórios
export type IdDoRelatorio =
  | 'producao_medico'
  | 'producao_convenio'
  | 'producao_sala'
  | 'faltas'
  | 'glosas'
  | 'financeiro'
  | 'estoque';

export interface RelatorioDoCatalogo {
  id: IdDoRelatorio;
  grupo: string;
  titulo: string;
  descricao: string;
  dado_de_saude: boolean;
}

export interface LinhaDoRelatorio {
  a: string;
  b: string;
  valor: number;
  c: string;
}

export interface PreviaDoRelatorio {
  id: IdDoRelatorio;
  titulo: string;
  de: string;
  ate: string;
  colunas: string[];
  linhas: LinhaDoRelatorio[];
  dado_de_saude: boolean;
  gerado_em: string;
}

export interface ExportacaoFeita {
  codigo: string;
  relatorio: string;
  titulo: string;
  formato: string;
  de: string;
  ate: string;
  conteudo: 'sem_dado_de_saude' | 'pseudonimizado';
  linhas: number;
  sha256: string;
  quem: string;
  quando: string;
}
