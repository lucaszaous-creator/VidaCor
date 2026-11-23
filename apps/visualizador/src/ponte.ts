// Ponte com o Rust (src-tauri/src/comandos.rs). A tela só pede: abrir exame (o diálogo é do Rust),
// ver um quadro com uma janela, exportar. Tudo o que decide (dados do PDF, um paciente por PDF,
// nomes de arquivo, registro) fica no Rust (G1).
import { invoke } from '@tauri-apps/api/core';

export interface Janela {
  centro: number;
  largura: number;
}

export interface Paciente {
  nome: string;
  /** dd/mm/aaaa; nulo quando o exame não traz. */
  nascimento: string | null;
  sexo: string;
}

export interface ImagemResumo {
  numero: number | null;
  quadros: number;
}

export interface Serie {
  numero: number | null;
  descricao: string;
  modalidade: string;
  imagens: ImagemResumo[];
}

export interface Estudo {
  paciente: Paciente;
  descricao: string;
  data: string | null;
  atendimento: string;
  solicitante: string;
  modalidades: string[];
  series: Serie[];
}

export interface Ignorado {
  nome: string;
  motivo: string;
}

export interface Resumo {
  abertura: number;
  estudos: Estudo[];
  ignorados: Ignorado[];
}

export interface JanelaPronta extends Janela {
  nome: string;
}

export interface InfoQuadro {
  largura: number;
  altura: number;
  cor: boolean;
  janelaPadrao: Janela | null;
  janelasDoArquivo: Janela[];
  prontas: JanelaPronta[];
}

export interface Posicao {
  estudo: number;
  serie: number;
  imagem: number;
  quadro: number;
}

export interface Item extends Posicao {
  /** Janela com que a imagem foi marcada; nula na imagem colorida. */
  janela: Janela | null;
}

export interface Acao {
  id: string;
  rotulo: string;
  atalho?: string | null;
  estado: 'disponivel' | 'indisponivel';
  motivo?: string | null;
}

export interface Resultado {
  quantidade: number;
  destino: string;
}

export interface Pixels {
  largura: number;
  altura: number;
  rgba: Uint8ClampedArray<ArrayBuffer>;
}

/** Bytes do comando `pixels`: largura e altura (u32 LE), canais (1 ou 3) e os pixels. */
export function decodificarPixels(buffer: ArrayBuffer): Pixels {
  const vista = new DataView(buffer);
  const largura = vista.getUint32(0, true);
  const altura = vista.getUint32(4, true);
  const canais = vista.getUint8(8);
  const origem = new Uint8Array(buffer, 9);
  const total = largura * altura;
  if ((canais !== 1 && canais !== 3) || origem.length < total * canais) {
    throw new Error('A imagem chegou incompleta. Abra o exame de novo.');
  }
  const rgba = new Uint8ClampedArray(total * 4);
  for (let i = 0; i < total; i++) {
    const o = i * canais;
    const d = i * 4;
    const r = origem[o] ?? 0;
    rgba[d] = r;
    rgba[d + 1] = canais === 1 ? r : (origem[o + 1] ?? 0);
    rgba[d + 2] = canais === 1 ? r : (origem[o + 2] ?? 0);
    rgba[d + 3] = 255;
  }
  return { largura, altura, rgba };
}

export interface Ponte {
  abrirPasta: () => Promise<Resumo | null>;
  abrirArquivo: () => Promise<Resumo | null>;
  quadro: (abertura: number, p: Posicao) => Promise<InfoQuadro>;
  pixels: (abertura: number, p: Posicao, janela: Janela | null) => Promise<Pixels>;
  acoesDaImagem: (marcada: boolean, totalMarcadas: number) => Promise<Acao[]>;
  exportarJpeg: (abertura: number, itens: Item[]) => Promise<Resultado | null>;
  gerarPdf: (abertura: number, itens: Item[], porPagina: number) => Promise<Resultado | null>;
}

export const ponteTauri: Ponte = {
  abrirPasta: () => invoke<Resumo | null>('abrir_pasta'),
  abrirArquivo: () => invoke<Resumo | null>('abrir_arquivo'),
  quadro: (abertura, p) => invoke<InfoQuadro>('quadro', { abertura, ...p }),
  pixels: async (abertura, p, janela) => decodificarPixels(await invoke<ArrayBuffer>('pixels', { abertura, ...p, janela })),
  acoesDaImagem: (marcada, totalMarcadas) => invoke<Acao[]>('acoes_da_imagem', { marcada, totalMarcadas }),
  exportarJpeg: (abertura, itens) => invoke<Resultado | null>('exportar_jpeg', { abertura, itens }),
  gerarPdf: (abertura, itens, porPagina) => invoke<Resultado | null>('gerar_pdf', { abertura, itens, porPagina }),
};

/** Mensagem do Rust (texto) ou de uma falha inesperada, sempre dizendo o que fazer (R8). */
export function mensagemDeErro(e: unknown): string {
  if (typeof e === 'string' && e !== '') {
    return e;
  }
  if (e instanceof Error && e.message !== '') {
    return e.message;
  }
  return 'Algo deu errado no programa. Feche e abra o Visualizador de novo; se continuar, chame o suporte de TI.';
}
