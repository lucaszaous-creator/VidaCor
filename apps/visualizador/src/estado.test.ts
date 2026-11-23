import { alternar, estudosNaSelecao, LIMITE_DE_ITENS, posicoesDaSerie, rotuloDaJanela } from './estado';
import { decodificarPixels, mensagemDeErro, type Item } from './ponte';

const item = (imagem: number, estudo = 0): Item => ({ estudo, serie: 0, imagem, quadro: 0, janela: null });

test('marcar, desmarcar e o limite', () => {
  let s = alternar([], item(1));
  expect(s).toHaveLength(1);
  s = alternar(s, item(1));
  expect(s).toHaveLength(0);
  const cheio = Array.from({ length: LIMITE_DE_ITENS }, (_, i) => item(i));
  expect(alternar(cheio, item(9999))).toHaveLength(LIMITE_DE_ITENS);
  expect(estudosNaSelecao([item(1, 0), item(2, 1), item(3, 0)])).toEqual([0, 1]);
});

test('quadros do cine entram em sequência', () => {
  const serie = { numero: 1, descricao: '', modalidade: 'US', paginasNoPdf: 1, imagens: [{ numero: 1, quadros: 3 }, { numero: 2, quadros: 1 }] };
  expect(posicoesDaSerie(serie)).toEqual([
    { imagem: 0, quadro: 0 },
    { imagem: 0, quadro: 1 },
    { imagem: 0, quadro: 2 },
    { imagem: 1, quadro: 0 },
  ]);
  expect(rotuloDaJanela({ centro: 40, largura: 400.25 })).toBe('40/400.3');
});

test('pixels cinza e coloridos viram RGBA; incompletos são recusados', () => {
  const cab = (l: number, a: number, c: number, px: number[]) => {
    const b = new ArrayBuffer(9 + px.length);
    const v = new DataView(b);
    v.setUint32(0, l, true);
    v.setUint32(4, a, true);
    v.setUint8(8, c);
    new Uint8Array(b, 9).set(px);
    return b;
  };
  expect(Array.from(decodificarPixels(cab(2, 1, 1, [10, 200])).rgba)).toEqual([10, 10, 10, 255, 200, 200, 200, 255]);
  expect(Array.from(decodificarPixels(cab(1, 1, 3, [1, 2, 3])).rgba)).toEqual([1, 2, 3, 255]);
  expect(() => decodificarPixels(cab(2, 2, 1, [1]))).toThrow(/incompleta/);
  expect(mensagemDeErro('o exame foi fechado')).toBe('o exame foi fechado');
  expect(mensagemDeErro(42)).toMatch(/suporte de TI/);
});
