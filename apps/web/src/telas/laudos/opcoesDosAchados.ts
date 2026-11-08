import type {
  CategoriaBirads,
  ComposicaoTirads,
  EcogenicidadeTirads,
  FocoTirads,
  FormaBirads,
  FormaTirads,
  LadoBirads,
  MargemBirads,
  MargemTirads,
  OrientacaoBirads,
  PadraoEcogenicoBirads,
  PosteriorBirads,
} from '@vidacor/api-client';
import type { CorDoChip } from '@vidacor/design-system';

/* Rótulos das opções do laudo estruturado (Laudo-Estruturado.dc.html). Só texto de tela: pontos,
   categoria e conduta vêm do servidor (G1). */

interface Opcao<T extends string> {
  valor: T;
  rotulo: string;
}

export const COMPOSICAO: Opcao<ComposicaoTirads>[] = [
  { valor: 'cistica_ou_quase_cistica', rotulo: 'Cístico ou quase todo cístico' },
  { valor: 'espongiforme', rotulo: 'Espongiforme' },
  { valor: 'mista_cistica_e_solida', rotulo: 'Misto' },
  { valor: 'solida_ou_quase_solida', rotulo: 'Sólido' },
  { valor: 'indeterminada_por_calcificacao', rotulo: 'Indeterminada (calcificação)' },
];

export const ECOGENICIDADE: Opcao<EcogenicidadeTirads>[] = [
  { valor: 'anecoica', rotulo: 'Anecoico' },
  { valor: 'hiper_ou_isoecoica', rotulo: 'Hiper ou isoecoico' },
  { valor: 'hipoecoica', rotulo: 'Hipoecoico' },
  { valor: 'muito_hipoecoica', rotulo: 'Muito hipoecoico' },
  { valor: 'indeterminada', rotulo: 'Indeterminada' },
];

export const FORMA_TIRADS: Opcao<FormaTirads>[] = [
  { valor: 'mais_largo_que_alto', rotulo: 'Mais largo que alto' },
  { valor: 'mais_alto_que_largo', rotulo: 'Mais alto que largo' },
];

export const MARGEM_TIRADS: Opcao<MargemTirads>[] = [
  { valor: 'lisa', rotulo: 'Lisas' },
  { valor: 'mal_definida', rotulo: 'Mal definidas' },
  { valor: 'lobulada_ou_irregular', rotulo: 'Lobuladas ou irregulares' },
  { valor: 'extensao_extratireoidiana', rotulo: 'Extensão extratireoidiana' },
  { valor: 'indeterminada', rotulo: 'Indeterminadas' },
];

export const FOCOS: Opcao<FocoTirads>[] = [
  { valor: 'nenhum_ou_cauda_de_cometa', rotulo: 'Nenhum ou cauda de cometa' },
  { valor: 'macrocalcificacoes', rotulo: 'Macrocalcificações' },
  { valor: 'calcificacoes_perifericas', rotulo: 'Periféricos' },
  { valor: 'focos_puntiformes', rotulo: 'Puntiformes' },
];

export const LADO: Opcao<LadoBirads>[] = [
  { valor: 'direita', rotulo: 'Mama direita' },
  { valor: 'esquerda', rotulo: 'Mama esquerda' },
];

export const CATEGORIA_BIRADS: Opcao<CategoriaBirads>[] = [
  { valor: '0', rotulo: '0' },
  { valor: '1', rotulo: '1' },
  { valor: '2', rotulo: '2' },
  { valor: '3', rotulo: '3' },
  { valor: '4A', rotulo: '4A' },
  { valor: '4B', rotulo: '4B' },
  { valor: '4C', rotulo: '4C' },
  { valor: '5', rotulo: '5' },
  { valor: '6', rotulo: '6' },
];

export const FORMA_BIRADS: Opcao<FormaBirads>[] = [
  { valor: 'oval', rotulo: 'Oval' },
  { valor: 'redonda', rotulo: 'Redonda' },
  { valor: 'irregular', rotulo: 'Irregular' },
];

export const ORIENTACAO: Opcao<OrientacaoBirads>[] = [
  { valor: 'paralela', rotulo: 'Paralela' },
  { valor: 'nao_paralela', rotulo: 'Não paralela' },
];

export const MARGEM_BIRADS: Opcao<MargemBirads>[] = [
  { valor: 'circunscrita', rotulo: 'Circunscrita' },
  { valor: 'indistinta', rotulo: 'Indistinta' },
  { valor: 'angular', rotulo: 'Angular' },
  { valor: 'microlobulada', rotulo: 'Microlobulada' },
  { valor: 'espiculada', rotulo: 'Espiculada' },
];

export const PADRAO: Opcao<PadraoEcogenicoBirads>[] = [
  { valor: 'anecoico', rotulo: 'Anecoico' },
  { valor: 'hiperecoico', rotulo: 'Hiperecoico' },
  { valor: 'complexo_cistico_e_solido', rotulo: 'Complexo cístico e sólido' },
  { valor: 'hipoecoico', rotulo: 'Hipoecoico' },
  { valor: 'isoecoico', rotulo: 'Isoecoico' },
  { valor: 'heterogeneo', rotulo: 'Heterogêneo' },
];

export const POSTERIOR: Opcao<PosteriorBirads>[] = [
  { valor: 'nenhum', rotulo: 'Nenhum' },
  { valor: 'reforco', rotulo: 'Reforço' },
  { valor: 'sombra', rotulo: 'Sombra' },
  { valor: 'combinado', rotulo: 'Combinado' },
];

/** Cor do chip pela categoria devolvida pelo servidor (só aparência). */
export function corDaCategoria(categoria: string): CorDoChip {
  if (['TR1', 'TR2', '1', '2'].includes(categoria)) return 'verde';
  if (['TR3', 'TR4', '0', '3'].includes(categoria)) return 'ambar';
  if (['TR5', '4A', '4B', '4C', '5', '6'].includes(categoria)) return 'rosa';
  return 'cinza';
}

/** Medida em milímetros com vírgula ("18 mm", "8,5 mm"). */
export function formatarMilimetros(mm: number | null | undefined): string {
  if (mm === null || mm === undefined) return 'sem medida';
  return `${mm.toLocaleString('pt-BR', { maximumFractionDigits: 1 })} mm`;
}
