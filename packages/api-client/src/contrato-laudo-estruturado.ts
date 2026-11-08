/**
 * Laudo estruturado com classificações (docs/22 E2, migration 0040, DEC-2026-11-08-64): achados
 * TI-RADS e BI-RADS ligados à versão do laudo. A tela envia só os descritores; pontos, categoria,
 * conduta e texto vêm do servidor, com a versão da regra (G1). Conferido contra o OpenAPI em
 * `conformidade.ts`.
 */

export type SistemaClassificacao = 'tirads' | 'birads';

export type ComposicaoTirads =
  | 'cistica_ou_quase_cistica'
  | 'espongiforme'
  | 'mista_cistica_e_solida'
  | 'solida_ou_quase_solida'
  | 'indeterminada_por_calcificacao';
export type EcogenicidadeTirads = 'anecoica' | 'hiper_ou_isoecoica' | 'hipoecoica' | 'muito_hipoecoica' | 'indeterminada';
export type FormaTirads = 'mais_largo_que_alto' | 'mais_alto_que_largo';
export type MargemTirads = 'lisa' | 'mal_definida' | 'lobulada_ou_irregular' | 'extensao_extratireoidiana' | 'indeterminada';
export type FocoTirads = 'nenhum_ou_cauda_de_cometa' | 'macrocalcificacoes' | 'calcificacoes_perifericas' | 'focos_puntiformes';

export interface DescritoresTirads {
  composicao: ComposicaoTirads;
  ecogenicidade: EcogenicidadeTirads;
  forma: FormaTirads;
  margem: MargemTirads;
  /** Pode ter mais de um; vazio vale "nenhum". */
  focos: FocoTirads[];
}

export type LadoBirads = 'direita' | 'esquerda';
export type FormaBirads = 'oval' | 'redonda' | 'irregular';
export type OrientacaoBirads = 'paralela' | 'nao_paralela';
export type MargemBirads = 'circunscrita' | 'indistinta' | 'angular' | 'microlobulada' | 'espiculada';
export type PadraoEcogenicoBirads = 'anecoico' | 'hiperecoico' | 'complexo_cistico_e_solido' | 'hipoecoico' | 'isoecoico' | 'heterogeneo';
export type PosteriorBirads = 'nenhum' | 'reforco' | 'sombra' | 'combinado';
export type CategoriaBirads = '0' | '1' | '2' | '3' | '4A' | '4B' | '4C' | '5' | '6';

export interface MassaBirads {
  forma: FormaBirads;
  orientacao: OrientacaoBirads;
  margens: MargemBirads[];
  padrao_ecogenico: PadraoEcogenicoBirads;
  posterior: PosteriorBirads;
}

export interface DescritoresBirads {
  lado: LadoBirads;
  /** Categoria final do médico; o servidor confere a coerência com a massa. */
  categoria_atribuida: CategoriaBirads;
  biopsia_maligna_confirmada: boolean;
  massa?: MassaBirads | null;
}

/** Corpo de `POST /v1/laudos/{id}/achados` (com `Idempotency-Key`). Sem categoria: quem calcula é o servidor. */
export interface NovoAchado {
  sistema: SistemaClassificacao;
  localizacao: string;
  maior_diametro_mm?: number | null;
  tirads?: DescritoresTirads | null;
  birads?: DescritoresBirads | null;
}

export interface AchadoEstruturado {
  id: string;
  codigo: string;
  numero: number;
  /** N1, N2... */
  rotulo: string;
  versao_numero: number;
  sistema: SistemaClassificacao;
  localizacao: string;
  maior_diametro_mm?: number | null;
  tirads?: DescritoresTirads | null;
  birads?: DescritoresBirads | null;
  pontos?: number | null;
  /** TR1 a TR5 ou 0 a 6 (com 4A, 4B, 4C), calculada ou conferida pelo servidor. */
  categoria: string;
  categoria_nome: string;
  conduta: string;
  /** Ex.: TIRADS-ACR-2017.1. */
  regra_versao: string;
  texto_achado: string;
  texto_conclusao: string;
  criado_em: string;
}

export interface RegraVigente {
  sistema: SistemaClassificacao;
  versao: string;
}

export interface AchadosDoLaudo {
  laudo_id: string;
  versao_numero: number;
  /** Só o médico atribuído com versão em rascunho edita. */
  editavel: boolean;
  achados: AchadoEstruturado[];
  texto_achados: string;
  texto_conclusao: string;
  regras_vigentes: RegraVigente[];
}
