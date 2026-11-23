// Ponte falsa para os testes da tela: a regra de verdade é provada nos testes do Rust (G4);
// aqui só se confere o que a tela mostra e o que ela pede.
import { vi } from 'vitest';
import type { InfoQuadro, Ponte, Resumo } from '../src/ponte';

export const RESUMO: Resumo = {
  abertura: 1,
  estudos: [
    {
      paciente: { nome: 'ANA MARIA FICTICIO', nascimento: '01/01/1980', sexo: 'F' },
      descricao: 'TC CRANIO',
      data: '01/01/2026',
      atendimento: 'TESTE0001',
      solicitante: 'MEDICO FICTICIO',
      modalidades: ['CT'],
      series: [
        { numero: 2, descricao: 'PARTES MOLES', modalidade: 'CT', imagens: [{ numero: 3, quadros: 1 }, { numero: 2, quadros: 1 }, { numero: 1, quadros: 1 }] },
        { numero: 3, descricao: 'OSSO', modalidade: 'CT', imagens: [{ numero: 1, quadros: 1 }] },
      ],
    },
    {
      paciente: { nome: 'JOÃO PEDRO FICTICIO', nascimento: '15/06/1975', sexo: 'M' },
      descricao: 'TC ABDOME',
      data: '01/01/2026',
      atendimento: 'TESTE0006',
      solicitante: '',
      modalidades: ['CT'],
      series: [{ numero: 1, descricao: 'PORTAL', modalidade: 'CT', imagens: [{ numero: 1, quadros: 1 }] }],
    },
  ],
  ignorados: [{ nome: 'nao_dicom.txt', motivo: 'não é um arquivo DICOM' }],
};

const INFO: InfoQuadro = {
  largura: 4,
  altura: 4,
  cor: false,
  janelaPadrao: { centro: 35, largura: 100 },
  janelasDoArquivo: [{ centro: 35, largura: 100 }],
  prontas: [
    { nome: 'Partes moles', centro: 40, largura: 400 },
    { nome: 'Cérebro', centro: 40, largura: 80 },
    { nome: 'Osso', centro: 500, largura: 2000 },
    { nome: 'Pulmão', centro: -600, largura: 1500 },
  ],
};

export function ponteFalsa(sobrescrever: Partial<Ponte> = {}) {
  const p = {
    abrirPasta: vi.fn(() => Promise.resolve<Resumo | null>(RESUMO)),
    abrirArquivo: vi.fn(() => Promise.resolve<Resumo | null>(RESUMO)),
    quadro: vi.fn(() => Promise.resolve(INFO)),
    pixels: vi.fn(() => Promise.resolve({ largura: 4, altura: 4, rgba: new Uint8ClampedArray(64) })),
    acoesDaImagem: vi.fn((marcada: boolean) =>
      Promise.resolve([
        { id: marcada ? 'desmarcar' : 'marcar', rotulo: marcada ? 'Tirar da exportação' : 'Marcar para exportar', atalho: 'M', estado: 'disponivel' as const },
        { id: 'salvar-jpeg', rotulo: 'Salvar esta imagem em JPEG', estado: 'disponivel' as const },
      ]),
    ),
    exportarJpeg: vi.fn(() => Promise.resolve({ quantidade: 1, destino: 'C:\\Exportados' })),
    gerarPdf: vi.fn(() => Promise.resolve({ quantidade: 1, destino: 'C:\\Exportados\\imagens-exame-TESTE0001.pdf' })),
    ...sobrescrever,
  };
  return p;
}
