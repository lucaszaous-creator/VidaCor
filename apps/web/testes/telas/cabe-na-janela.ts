/**
 * "A tela cabe na janela" (docs/05, E2E-6): abre cada tela em 1440×900 e 1280×800 no Chromium e
 * reprova se a página rolar na vertical ou na horizontal, ou se a grade da agenda rolar para o lado.
 * Tira prints em testes/prints/ (fora do commit) para comparar com os mockups.
 *
 * O núcleo é simulado por `page.route` com os dados fictícios (só a tela é verificada aqui).
 * Roda direto no Node 22 (tipos apagados): `pnpm --filter @vidacor/web test:telas`.
 */
import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, type Page, type Route } from 'playwright';
import { createServer } from 'vite';
import { agendaDoDia, ANTONIO, atendimentoDe, DATA_DOS_TESTES, SALAS, SESSAO } from '../dados/ficticios.ts';

const aqui = dirname(fileURLToPath(import.meta.url));
const raizDoApp = join(aqui, '..', '..');
const pastaDePrints = join(aqui, '..', 'prints');
mkdirSync(pastaDePrints, { recursive: true });

const JANELAS = [
  { largura: 1440, altura: 900 },
  { largura: 1280, altura: 800 },
];

const falhas: string[] = [];
let conferidas = 0;

function json(route: Route, corpo: unknown, status = 200): Promise<void> {
  return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(corpo) });
}

async function simularNucleo(page: Page): Promise<void> {
  let checkinFeito = false;
  await page.route('**/v1/**', async (route) => {
    const url = new URL(route.request().url());
    const metodo = route.request().method();
    const caminho = url.pathname;
    if (caminho === '/v1/saude') return json(route, { situacao: 'ok' });
    if (caminho === '/v1/sessao') return json(route, SESSAO);
    if (caminho === '/v1/salas') return json(route, SALAS);
    if (caminho === '/v1/eventos') {
      return route.fulfill({ status: 200, contentType: 'text/event-stream', body: 'retry: 600000\n\n' });
    }
    if (caminho === '/v1/agenda') {
      const agenda = agendaDoDia(url.searchParams.get('data') ?? '', url.searchParams.get('sala_id'));
      if (checkinFeito) {
        for (const i of agenda.itens) {
          if (i.id === ANTONIO.id) {
            i.situacao = 'em_atendimento';
            i.atendimento_id = atendimentoDe(ANTONIO, 1).id;
          }
        }
      }
      return json(route, agenda);
    }
    if (caminho === '/v1/atendimentos' && metodo === 'POST') {
      checkinFeito = true;
      const a = atendimentoDe(ANTONIO, 1);
      return json(route, { ...a, estudo: undefined }, 201);
    }
    if (caminho.startsWith('/v1/atendimentos/')) return json(route, atendimentoDe(ANTONIO, 1));
    return route.fulfill({ status: 404, contentType: 'application/problem+json', body: '{}' });
  });
}

async function conferir(page: Page, nome: string, largura: number, altura: number): Promise<void> {
  await page.evaluate(() => document.fonts.ready);
  const m = await page.evaluate(() => {
    const s = document.scrollingElement ?? document.documentElement;
    const grade = document.querySelector('.card.pane > .scroll');
    return {
      alturaDaPagina: s.scrollHeight,
      larguraDaPagina: s.scrollWidth,
      alturaDaJanela: window.innerHeight,
      larguraDaJanela: window.innerWidth,
      gradeLateral: grade === null ? 0 : grade.scrollWidth - grade.clientWidth,
    };
  });
  conferidas += 1;
  const rotulo = `${nome} em ${String(largura)}x${String(altura)}`;
  if (m.alturaDaPagina > m.alturaDaJanela) {
    falhas.push(`${rotulo}: a página rola na vertical (${String(m.alturaDaPagina)} > ${String(m.alturaDaJanela)})`);
  }
  if (m.larguraDaPagina > m.larguraDaJanela) {
    falhas.push(`${rotulo}: rolagem lateral na página (${String(m.larguraDaPagina)} > ${String(m.larguraDaJanela)})`);
  }
  if (m.gradeLateral > 0) {
    falhas.push(`${rotulo}: a grade da agenda rola para o lado (${String(m.gradeLateral)} px)`);
  }
  await page.screenshot({ path: join(pastaDePrints, `${nome}-${String(largura)}x${String(altura)}.png`) });
}

async function esperar(page: Page, papel: Parameters<Page['getByRole']>[0], nome: string | RegExp): Promise<void> {
  await page.getByRole(papel, { name: nome }).first().waitFor({ state: 'visible', timeout: 10_000 });
}

const servidor = await createServer({
  root: raizDoApp,
  configFile: join(raizDoApp, 'vite.config.ts'),
  logLevel: 'error',
  server: { port: 5199, strictPort: false, proxy: {} },
});
await servidor.listen();
const base = servidor.resolvedUrls?.local[0] ?? 'http://localhost:5199/';
const navegador = await chromium.launch();

try {
  for (const { largura, altura } of JANELAS) {
    const contexto = await navegador.newContext({
      viewport: { width: largura, height: altura },
      locale: 'pt-BR',
      timezoneId: 'America/Sao_Paulo',
    });
    const page = await contexto.newPage();
    page.on('pageerror', (e) => falhas.push(`erro de script em ${String(largura)}x${String(altura)}: ${e.message}`));
    await simularNucleo(page);

    await page.goto(new URL('/entrar', base).toString());
    await esperar(page, 'heading', 'Entrar');
    await page.getByText('Servidor local conectado').waitFor();
    await conferir(page, '01-entrar', largura, altura);

    await page.goto(new URL(`/agenda?data=${DATA_DOS_TESTES}`, base).toString());
    await esperar(page, 'heading', 'Sexta, 6 de novembro');
    await esperar(page, 'button', /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954/);
    await conferir(page, '02-agenda', largura, altura);

    const antonio = page.getByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954/ });
    await antonio.focus();
    await page.keyboard.press('Shift+F10');
    await esperar(page, 'menu', 'Ações de Antônio Carlos Ribeiro');
    await conferir(page, '03-menu-de-contexto', largura, altura);
    await page.keyboard.press('Escape');

    await antonio.click();
    await esperar(page, 'complementary', 'Detalhe do agendamento');
    await conferir(page, '04-detalhe', largura, altura);

    await page.getByRole('button', { name: 'Fazer check-in', exact: true }).click();
    await esperar(page, 'complementary', 'Check-in do paciente');
    await page.getByRole('textbox', { name: 'Nome completo' }).fill('Antônio Carlos Ribeiro');
    await page.getByRole('spinbutton').first().click();
    await page.keyboard.type('15061954');
    await conferir(page, '05-check-in', largura, altura);

    await page.getByRole('button', { name: 'Conferir e fazer check-in' }).click();
    await esperar(page, 'alertdialog', 'Fazer o check-in de Antônio Carlos Ribeiro?');
    await conferir(page, '06-confirmacao', largura, altura);

    await page.getByRole('alertdialog').getByRole('button', { name: 'Fazer check-in' }).click();
    await esperar(page, 'complementary', 'Check-in feito');
    await page.getByText('ATD-2026-118791').first().waitFor();
    await conferir(page, '07-check-in-feito', largura, altura);

    await page.getByRole('button', { name: 'Recolher menu' }).click();
    await esperar(page, 'button', 'Expandir menu');
    await conferir(page, '08-menu-recolhido', largura, altura);

    await contexto.close();
  }
} finally {
  await navegador.close();
  await servidor.close();
}

if (falhas.length > 0) {
  console.error(`Reprovado: ${String(falhas.length)} problema(s) em ${String(conferidas)} conferências.`);
  for (const f of falhas) console.error(`- ${f}`);
  process.exit(1);
}
console.log(`Aprovado: ${String(conferidas)} conferências (8 telas em 2 janelas), sem rolagem de página nem lateral. Prints em ${pastaDePrints}`);
