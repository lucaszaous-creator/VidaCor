// Estado da tela que não é regra de negócio: posição na série e imagens marcadas para exportar.
import type { Item, Posicao, Serie } from './ponte';

/** Mesmo limite do Rust (MAX_IMAGENS_PDF); o Rust confere de novo. */
export const LIMITE_DE_ITENS = 600;

export function chave(p: Posicao): string {
  return `${String(p.estudo)}-${String(p.serie)}-${String(p.imagem)}-${String(p.quadro)}`;
}

/** Posições de uma série, com os quadros do cine em sequência (imagem, quadro). */
export function posicoesDaSerie(serie: Serie): { imagem: number; quadro: number }[] {
  return serie.imagens.flatMap((img, imagem) => Array.from({ length: Math.max(1, img.quadros) }, (_, quadro) => ({ imagem, quadro })));
}

export function estaMarcada(selecao: Item[], p: Posicao): boolean {
  const k = chave(p);
  return selecao.some((i) => chave(i) === k);
}

/** Marca ou desmarca. Acima do limite não marca (a tela avisa pelo menu e pelo painel). */
export function alternar(selecao: Item[], item: Item): Item[] {
  const k = chave(item);
  if (selecao.some((i) => chave(i) === k)) {
    return selecao.filter((i) => chave(i) !== k);
  }
  if (selecao.length >= LIMITE_DE_ITENS) {
    return selecao;
  }
  return [...selecao, item];
}

/** Estudos diferentes na seleção: o PDF não sai (VIS-S1). O Rust confere de novo. */
export function estudosNaSelecao(selecao: Item[]): number[] {
  return [...new Set(selecao.map((i) => i.estudo))];
}

/** Valor com no máximo uma casa decimal, como na legenda do PDF. */
export function numero(n: number): string {
  return Number.isInteger(n) ? String(n) : n.toFixed(1);
}

export function rotuloDaJanela(j: { centro: number; largura: number }): string {
  return `${numero(j.centro)}/${numero(j.largura)}`;
}
