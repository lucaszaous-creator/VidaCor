/**
 * Modelos de laudo, frases rápidas e revisão por pares (Modelos-Laudo.dc.html e Revisao-Pares.dc.html;
 * DEC-2026-11-08-100 a 103). O servidor decide quem edita, aprova e revisa; a tela só mostra.
 * Conferido com o `openapi.json` em `conformidade.ts`.
 */
import type { PacienteResumo } from './contrato-m0';

export type TipoDeModelo = 'clinica' | 'pessoal';
export type SituacaoDaVersaoDeModelo = 'proposta' | 'publicada' | 'substituida' | 'recusada';

export interface VersaoDeModelo {
  numero: number;
  texto: string;
  motivo?: string | null;
  situacao: SituacaoDaVersaoDeModelo;
  proposta_por: string;
  decidido_por?: string | null;
  decidido_em?: string | null;
  motivo_decisao?: string | null;
  criado_em: string;
}

export interface ModeloDeLaudo {
  id: string;
  codigo: string;
  tipo: TipoDeModelo;
  nome: string;
  exame: string;
  modalidade: string;
  autor: string;
  ativo: boolean;
  motivo_desativacao?: string | null;
  versao_publicada?: number | null;
  proposta_em_aberto?: number | null;
  usos: number;
  pode_editar: boolean;
  pode_decidir: boolean;
  pode_desativar: boolean;
  versoes: VersaoDeModelo[];
}

export interface NovoModeloDeLaudo {
  tipo: TipoDeModelo;
  nome: string;
  exame: string;
  modalidade: string;
  texto: string;
  derivado_de?: string | null;
}

export interface NovaVersaoDeModelo {
  texto: string;
  motivo: string;
  versao_base?: number | null;
}

export interface DecisaoDeVersaoDeModelo {
  aprovar: boolean;
  motivo?: string | null;
}

export interface UsoDeModelo {
  modelo_codigo: string;
  numero: number;
  laudo_versao: number;
  texto: string;
}

export interface FraseDeLaudo {
  id: string;
  codigo: string;
  tipo: TipoDeModelo;
  atalho: string;
  texto: string;
  dono: string;
  substitui_codigo?: string | null;
  motivo?: string | null;
  criado_em: string;
  pode_editar: boolean;
}

export interface NovaFraseDeLaudo {
  tipo: TipoDeModelo;
  atalho: string;
  texto: string;
  substitui_id?: string | null;
  motivo?: string | null;
}

/** Escala RADPEER. */
export type ClasseRadpeer = '1' | '2a' | '2b' | '3a' | '3b';
export type SituacaoDaRevisao = 'designada' | 'concluida' | 'conflito_declarado';
export type PendenciaDaRevisao = 'aguarda_autor' | 'retificado' | 'mantido';

export interface ItemDaFilaDeRevisao {
  id: string;
  codigo: string;
  laudo_codigo: string;
  exame: string;
  situacao: SituacaoDaRevisao;
  classe?: ClasseRadpeer | null;
  designada_em: string;
  concluida_em?: string | null;
}

export interface CasoDeRevisao {
  id: string;
  codigo: string;
  situacao: SituacaoDaRevisao;
  laudo_id: string;
  laudo_codigo: string;
  exame: string;
  versao: number;
  paciente: PacienteResumo;
  texto?: string | null;
  assinada_em?: string | null;
  estudo_id?: string | null;
  autor?: string | null;
  classe?: ClasseRadpeer | null;
  comentario?: string | null;
  concluida_em?: string | null;
}

export interface ParecerDeRevisao {
  classe: ClasseRadpeer;
  comentario: string;
}

export interface RevisaoDoMeuLaudo {
  id: string;
  codigo: string;
  laudo_id: string;
  laudo_codigo: string;
  exame: string;
  versao: number;
  paciente: PacienteResumo;
  classe: ClasseRadpeer;
  comentario?: string | null;
  concluida_em: string;
  pendencia?: PendenciaDaRevisao | null;
  pendencia_motivo?: string | null;
  pendencia_resolvida_em?: string | null;
}

export interface RevisoesDoAutor {
  revisados: number;
  concordantes: number;
  concordancia_percentual?: number | null;
  pendentes: number;
  revisoes: RevisaoDoMeuLaudo[];
}

export interface DecisaoDePendencia {
  decisao: 'retificado' | 'mantido';
  motivo?: string | null;
}

export interface RevisaoDesignada {
  id: string;
  codigo: string;
  laudo_id: string;
  laudo_codigo: string;
  exame: string;
  revisor_id: string;
  revisor: string;
  modo: 'sorteio' | 'selecao';
  situacao: SituacaoDaRevisao;
  designada_em: string;
  motivo_conflito?: string | null;
}

export interface ResultadoDoSorteio {
  candidatos: number;
  designadas: RevisaoDesignada[];
  sem_revisor: number;
}

export interface PainelDeRevisao {
  dias: number;
  por_medico: {
    medico_id: string;
    medico: string;
    revisados: number;
    concordantes: number;
    concordancia_percentual?: number | null;
    discrepancias: number;
    pendentes: number;
  }[];
  discrepancias: {
    id: string;
    codigo: string;
    laudo_codigo: string;
    exame: string;
    classe: ClasseRadpeer;
    autor: string;
    pendencia?: PendenciaDaRevisao | null;
    concluida_em: string;
  }[];
  abertas: RevisaoDesignada[];
  conflitos: RevisaoDesignada[];
  medicos: { id: string; nome: string }[];
}
