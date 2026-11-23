/* global process, console, URL, window, document */
// "A tela cabe na janela" (docs/05) para o Visualizador: abre a página compilada (dist/) no Chromium
// com a ponte do Tauri simulada e exame fictício, em 1440×900 e 1280×800, e confere que a página não
// rola em nenhuma direção na tela inicial, com o exame aberto e com o diálogo do PDF.
// Uso: pnpm build && pnpm test:telas [pasta-para-prints]
// Sai com 1 se alguma tela rolar.
import { readFile } from 'node:fs/promises';
import { extname, join, resolve } from 'node:path';
import { chromium } from 'playwright';

const DIST = resolve(import.meta.dirname, '../../dist');
const PRINTS = process.argv[2];
const ORIGEM = 'http://visualizador.teste';
const TIPOS = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.woff2': 'font/woff2', '.woff': 'font/woff' };

// Ponte simulada: mesmo formato dos comandos de src-tauri/src/comandos.rs. Imagem fictícia (fantoma).
function ponteSimulada() {
  const estudo = (nome, nascimento, descricao, atendimento, series) => ({
    paciente: { nome, nascimento, sexo: 'F' },
    descricao,
    data: '01/01/2026',
    atendimento,
    solicitante: 'MEDICO FICTICIO',
    modalidades: ['CT'],
    series,
  });
  const serie = (numero, descricao, n) => ({
    numero,
    descricao,
    modalidade: 'CT',
    imagens: Array.from({ length: n }, (_, i) => ({ numero: i + 1, quadros: 1 })),
  });
  const resumo = {
    abertura: 1,
    estudos: [
      estudo('ANA MARIA FICTICIO DE OLIVEIRA SANTOS', '01/01/1980', 'TC CRANIO SEM CONTRASTE', 'TESTE0001', [
        serie(2, 'PARTES MOLES Head 2.0', 35),
        serie(3, 'OSSO Head 1.0', 70),
        serie(4, 'CORONAL MPR', 40),
        serie(5, 'SAGITAL MPR', 40),
      ]),
    ],
    ignorados: [{ nome: 'DICOMDIR', motivo: 'arquivo DICOM sem imagem (índice, laudo ou outro objeto)' }],
  };
  const fantoma = (l, janela) => {
    const b = new ArrayBuffer(9 + l * l);
    const v = new DataView(b);
    v.setUint32(0, l, true);
    v.setUint32(4, l, true);
    v.setUint8(8, 1);
    const px = new Uint8Array(b, 9);
    const k = 255 / Math.max(1, janela?.largura ?? 100);
    for (let y = 0; y < l; y++) {
      for (let x = 0; x < l; x++) {
        const dx = x - l / 2;
        const dy = y - l / 2;
        const r = Math.sqrt(dx * dx + dy * 1.3 * dy * 1.3);
        const hu = r < l * 0.38 ? (r > l * 0.34 ? 900 : 35 + 10 * Math.sin(x / 9)) : -1000;
        px[y * l + x] = Math.max(0, Math.min(255, ((hu - ((janela?.centro ?? 35) - (janela?.largura ?? 100) / 2)) * k) | 0));
      }
    }
    return b;
  };
  window.__TAURI_INTERNALS__ = {
    transformCallback: () => 0,
    invoke: async (cmd, args) => {
      switch (cmd) {
        case 'abrir_pasta':
        case 'abrir_arquivo':
          return resumo;
        case 'quadro':
          return {
            largura: 512,
            altura: 512,
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
        case 'pixels':
          return fantoma(512, args.janela);
        case 'acoes_da_imagem':
          return [{ id: args.marcada ? 'desmarcar' : 'marcar', rotulo: args.marcada ? 'Tirar da exportação' : 'Marcar para exportar', atalho: 'M', estado: 'disponivel' }];
        default:
          return null;
      }
    },
  };
}

async function rola(pagina) {
  return pagina.evaluate(() => {
    const el = document.scrollingElement;
    return { vertical: el.scrollHeight > window.innerHeight + 1, lateral: el.scrollWidth > window.innerWidth + 1 };
  });
}

const navegador = await chromium.launch();
let falhas = 0;
for (const [largura, altura] of [
  [1440, 900],
  [1280, 800],
]) {
  const pagina = await navegador.newPage({ viewport: { width: largura, height: altura }, deviceScaleFactor: PRINTS ? 2 : 1 });
  await pagina.route(`${ORIGEM}/**`, async (rota) => {
    const caminho = new URL(rota.request().url()).pathname;
    const arquivo = join(DIST, caminho === '/' ? 'index.html' : caminho);
    await rota.fulfill({ body: await readFile(arquivo), contentType: TIPOS[extname(arquivo)] ?? 'application/octet-stream' });
  });
  await pagina.addInitScript(ponteSimulada);
  await pagina.goto(`${ORIGEM}/`);
  const etapas = [
    ['inicio', async () => pagina.getByRole('button', { name: 'Abrir pasta' }).waitFor()],
    [
      'exame',
      async () => {
        await pagina.getByRole('button', { name: 'Abrir pasta' }).click();
        await pagina.getByLabel(/Paciente em foco/).waitFor();
        for (let i = 0; i < 8; i++) {
          await pagina.getByRole('button', { name: /Marcar para exportar|Tirar da exportação/ }).click();
          await pagina.getByLabel(/Setas trocam a imagem/).press('ArrowDown');
        }
        await pagina.getByRole('button', { name: 'Marcar para exportar (M)' }).waitFor();
      },
    ],
    [
      'confirmar-pdf',
      async () => {
        await pagina.getByRole('button', { name: 'Gerar PDF do paciente' }).click();
        await pagina.getByRole('alertdialog').waitFor();
      },
    ],
  ];
  for (const [nome, preparar] of etapas) {
    await preparar();
    const r = await rola(pagina);
    const ok = !r.vertical && !r.lateral;
    if (!ok) {
      falhas += 1;
    }
    console.log(`${ok ? 'cabe' : 'ROLA'}  ${String(largura)}x${String(altura)}  ${nome}${r.vertical ? ' (vertical)' : ''}${r.lateral ? ' (lateral)' : ''}`);
    if (PRINTS) {
      await pagina.screenshot({ path: join(PRINTS, `visualizador-${nome}-${String(largura)}x${String(altura)}.png`) });
    }
  }
  await pagina.close();
}
await navegador.close();
process.exit(falhas > 0 ? 1 : 0);
