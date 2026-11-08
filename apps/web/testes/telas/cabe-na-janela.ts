/**
 * "A tela cabe na janela" (docs/05, E2E-6): abre cada tela em 1440×900 e 1280×800 no Chromium e
 * reprova se a página rolar na vertical ou na horizontal, ou se algum painel (`.scroll`) rolar para o
 * lado. O longo rola dentro do painel, na vertical. Tira prints em testes/prints/ (fora do commit)
 * para comparar com os mockups.
 *
 * O núcleo é simulado por `page.route` com os dados fictícios (só a tela é verificada aqui).
 * Roda direto no Node 22 (tipos apagados): `pnpm --filter @vidacor/web test:telas`.
 */
import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, type Page } from 'playwright';
import { createServer } from 'vite';
import { ATENDIMENTO_VERA, LAUDO_ANTONIO_ID, PORTAL_LAUDO_ECO, SALA_1, SESSOES, SONIA } from '../dados/fase1.ts';
import { DATA_DOS_TESTES } from '../dados/ficticios.ts';
import { novoEstado, simularNucleo, type EstadoDaSimulacao } from './nucleo-playwright.ts';

const aqui = dirname(fileURLToPath(import.meta.url));
const raizDoApp = join(aqui, '..', '..');
const pastaDePrints = join(aqui, '..', 'prints');
mkdirSync(pastaDePrints, { recursive: true });

const JANELAS = [
  { largura: 1440, altura: 900 },
  { largura: 1280, altura: 800 },
];

const falhas: string[] = [];
const telas = new Set<string>();
let conferidas = 0;

async function conferir(page: Page, nome: string, largura: number, altura: number): Promise<void> {
  await page.evaluate(() => document.fonts.ready);
  const m = await page.evaluate(() => {
    const s = document.scrollingElement ?? document.documentElement;
    const paineis = [...document.querySelectorAll<HTMLElement>('.scroll')].map((e) => e.scrollWidth - e.clientWidth);
    return {
      alturaDaPagina: s.scrollHeight,
      larguraDaPagina: s.scrollWidth,
      alturaDaJanela: window.innerHeight,
      larguraDaJanela: window.innerWidth,
      painelLateral: Math.max(0, ...paineis),
    };
  });
  conferidas += 1;
  telas.add(nome);
  const rotulo = `${nome} em ${String(largura)}x${String(altura)}`;
  if (m.alturaDaPagina > m.alturaDaJanela) {
    falhas.push(`${rotulo}: a página rola na vertical (${String(m.alturaDaPagina)} > ${String(m.alturaDaJanela)})`);
  }
  if (m.larguraDaPagina > m.larguraDaJanela) {
    falhas.push(`${rotulo}: rolagem lateral na página (${String(m.larguraDaPagina)} > ${String(m.larguraDaJanela)})`);
  }
  if (m.painelLateral > 0) {
    falhas.push(`${rotulo}: um painel rola para o lado (${String(m.painelLateral)} px)`);
  }
  await page.screenshot({ path: join(pastaDePrints, `${nome}-${String(largura)}x${String(altura)}.png`) });
}

async function esperar(page: Page, papel: Parameters<Page['getByRole']>[0], nome: string | RegExp): Promise<void> {
  await page.getByRole(papel, { name: nome }).first().waitFor({ state: 'visible', timeout: 10_000 });
}

async function digitarData(page: Page, rotulo: string, digitos: string): Promise<void> {
  await page.getByRole('group', { name: rotulo }).getByRole('spinbutton').first().click();
  await page.keyboard.type(digitos);
}

type Passo = (page: Page, estado: EstadoDaSimulacao, ir: (caminho: string) => Promise<void>, ok: (nome: string) => Promise<void>) => Promise<void>;

/** M0: entrada, agenda e check-in pela gaveta. */
const m0: Passo = async (page, _estado, ir, ok) => {
  await ir('/entrar');
  await esperar(page, 'heading', 'Entrar');
  await page.getByText('Servidor local conectado').waitFor();
  await ok('01-entrar');

  await ir(`/agenda?data=${DATA_DOS_TESTES}`);
  await esperar(page, 'heading', 'Sexta, 6 de novembro');
  await esperar(page, 'button', /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954/);
  await ok('02-agenda');

  const antonio = page.getByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954/ });
  await antonio.focus();
  await page.keyboard.press('Shift+F10');
  await esperar(page, 'menu', 'Ações de Antônio Carlos Ribeiro');
  await ok('03-menu-de-contexto');
  await page.keyboard.press('Escape');

  await antonio.click();
  await esperar(page, 'complementary', 'Detalhe do agendamento');
  await ok('04-detalhe');

  await page.getByRole('button', { name: 'Fazer check-in', exact: true }).click();
  await esperar(page, 'complementary', 'Check-in do paciente');
  await page.getByRole('textbox', { name: 'Nome completo' }).fill('Antônio Carlos Ribeiro');
  await digitarData(page, 'Data de nascimento', '15061954');
  await page.getByText('Documento com foto conferido').click();
  await ok('05-check-in');

  await page.getByRole('button', { name: 'Conferir e fazer check-in' }).click();
  await esperar(page, 'alertdialog', 'Fazer o check-in de Antônio Carlos Ribeiro?');
  await ok('06-confirmacao');

  await page.getByRole('alertdialog').getByRole('button', { name: 'Fazer check-in' }).click();
  await esperar(page, 'complementary', 'Check-in feito');
  await page.getByText('ATD-2026-118791').first().waitFor();
  await ok('07-check-in-feito');

  await page.getByRole('button', { name: 'Recolher menu' }).click();
  await esperar(page, 'button', 'Expandir menu');
  await ok('08-menu-recolhido');
  await page.getByRole('button', { name: 'Expandir menu' }).click();
};

/** Fase 1: as telas dos 10 fluxos, nos estados que mais ocupam espaço. */
const fase1: Passo = async (page, estado, ir, ok) => {
  estado.sessao = SESSOES.recepcao;
  estado.checkinFeito = false;
  await ir(`/recepcao?data=${DATA_DOS_TESTES}`);
  await page.getByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954/ }).click();
  await page.getByText(/Homônimo hoje\./).waitFor();
  await page.getByRole('button', { name: 'Conferir e fazer check-in' }).click();
  await ok('09-recepcao-check-in');
  await page.getByRole('button', { name: /^Sônia Maria Castro, nascimento 03\/02\/1962/ }).click();
  await esperar(page, 'figure', /Pulseira ETQ-2026-018742/);
  await ok('10-recepcao-pulseira');
  await page.getByRole('button', { name: 'Reimprimir pulseira' }).click();
  await esperar(page, 'alertdialog', 'Reimprimir a pulseira de Sônia Maria Castro?');
  await ok('11-recepcao-reimpressao');
  await page.keyboard.press('Escape');

  estado.sessao = SESSOES.tecnico;
  await ir(`/sala?sala=${SALA_1.id}`);
  await esperar(page, 'button', /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954/);
  await page.getByRole('textbox', { name: /Código da pulseira/ }).fill('ETQ-2026-018742');
  await page.keyboard.press('Enter');
  await page.getByText(/Exame liberado\./).waitFor();
  await ok('12-sala-liberado');
  await page.getByRole('textbox', { name: /Código da pulseira/ }).fill('ETQ-2026-018755');
  await page.keyboard.press('Enter');
  await page.getByText(/encaminhe e bipe a próxima pulseira/).waitFor();
  await ok('13-sala-outra-sala');

  estado.sessao = SESSOES.medicaTc;
  await ir(`/triagem/${ATENDIMENTO_VERA.id}`);
  await esperar(page, 'textbox', 'Conduta');
  await ok('14-triagem-conduta');

  estado.sessao = SESSOES.recepcao;
  await ir('/agendamentos/novo');
  await page.getByRole('searchbox', { name: 'Buscar paciente por nome' }).fill('Antônio');
  await page.getByRole('button', { name: 'Antônio Carlos Ribeiro, nascimento 15/06/1954' }).click();
  await page.getByRole('combobox', { name: 'Sala' }).selectOption(SALA_1.id);
  await digitarData(page, 'Dia do exame', '07112026');
  await esperar(page, 'button', '09:00');
  await page.getByRole('button', { name: 'Agendar com a guia' }).click({ force: true });
  await ok('15-agendamento');

  estado.sessao = SESSOES.medico;
  estado.semVinculo = false;
  await ir(`/laudos/${LAUDO_ANTONIO_ID}`);
  await esperar(page, 'textbox', 'Texto do laudo, versão 1');
  await ok('16-laudo-rascunho');
  await page.getByRole('region', { name: 'Achados estruturados' }).scrollIntoViewIfNeeded();
  await esperar(page, 'button', 'Registrar achado');
  await ok('16b-laudo-achados-estruturados');
  await page.getByRole('button', { name: 'Assinar (ICP-Brasil)' }).click();
  await esperar(page, 'alertdialog', 'Assinar o laudo de Antônio Carlos Ribeiro (15/06/1954)?');
  await ok('17-laudo-assinatura');
  await page.keyboard.press('Escape');
  await ir(`/estudos?data=${DATA_DOS_TESTES}`);
  await page.getByRole('button', { name: /EST-2026-000081/ }).click();
  await esperar(page, 'combobox', 'Atendimento do dia');
  await ok('18a-estudos');
  estado.sessao = SESSOES.medicoSemVinculo;
  estado.semVinculo = true;
  await ir(`/laudos/${LAUDO_ANTONIO_ID}`);
  await esperar(page, 'textbox', 'Justificativa');
  await ok('18-laudo-sem-vinculo');
  estado.semVinculo = false;

  await ir('/portal');
  await esperar(page, 'region', 'Laudos publicados');
  await page.getByText('Retificado', { exact: true }).waitFor();
  await ok('19-portal');
  await ir(`/portal/laudos/${PORTAL_LAUDO_ECO.id}`);
  await esperar(page, 'article', 'Laudo de Ecocardiograma transtorácico');
  await ok('20-portal-laudo');
  await ir('/portal/link/token-expirado');
  await page.getByText(/Este link expirou\./).waitFor();
  await ok('21-portal-link-expirado');

  estado.sessao = SESSOES.caixa;
  estado.caixaAberto = false;
  await ir(`/caixa?data=${DATA_DOS_TESTES}`);
  await esperar(page, 'textbox', 'Fundo de troco (R$)');
  await ok('22-caixa-abrir');
  estado.caixaAberto = true;
  await ir(`/caixa?data=${DATA_DOS_TESTES}`);
  await page.getByRole('option', { name: /^Sônia Maria Castro/ }).waitFor({ state: 'attached' });
  await page.getByRole('combobox', { name: 'Paciente (check-in feito hoje)' }).selectOption(SONIA.atendimento_id ?? '');
  await page.getByRole('textbox', { name: 'Procedimentos (códigos TUSS)' }).fill('40901114');
  await page.getByRole('combobox', { name: 'Forma de pagamento' }).selectOption('pix');
  await page.getByRole('button', { name: 'Registrar recebimento' }).click();
  await esperar(page, 'table', 'Itens calculados pelo servidor');
  await ok('23-caixa-recebimento');

  estado.sessao = SESSOES.gerente;
  await ir('/caixa/pendencias');
  await page.getByRole('button', { name: /^Sônia Maria Castro, nascimento/ }).click();
  await esperar(page, 'heading', 'Desconto de 15,00% pedido por Carla Mendes');
  await ok('24-pendencias-desconto');
  await ir('/caixa/pendencias?aba=estornos');
  await page.getByRole('button', { name: /^Sônia Maria Castro, nascimento/ }).click();
  await esperar(page, 'heading', 'Estorno de R$ 380,00 pedido por Carla Mendes');
  await ok('25-pendencias-estorno');

  estado.sessao = SESSOES.recepcao;
  estado.contingenciaAberta = false;
  await ir('/pacientes/novo');
  await esperar(page, 'heading', 'Novo paciente');
  await ok('28-cadastro-paciente');
  estado.sessao = SESSOES.gerente;
  await ir('/estoque');
  await page.getByRole('button', { name: /Contraste iodado/ }).click();
  await esperar(page, 'region', 'Detalhe do item');
  await ok('29-estoque');
  await ir('/lastro');
  await page.getByRole('searchbox', { name: 'Buscar por qualquer código' }).fill('ATD-2026-000031');
  await page.getByRole('button', { name: 'Abrir cadeia' }).click();
  await esperar(page, 'heading', 'Sônia Maria Castro');
  await ok('30-lastro');
  await ir('/indicadores');
  await esperar(page, 'region', 'Resumo');
  await ok('31-indicadores');
  await ir('/financeiro');
  await esperar(page, 'region', 'Resumo do mês');
  await ok('32-financeiro');
  await ir('/usuarios');
  await page.getByRole('button', { name: /Dra\. Renata Alves/ }).click();
  await esperar(page, 'region', 'Detalhe do usuário');
  await ok('33-usuarios');
  estado.sessao = SESSOES.faturista;
  await ir('/faturamento');
  await page.getByRole('button', { name: /^Maria Helena Duarte, nascimento 12\/03\/1968/ }).first().click();
  await esperar(page, 'form', 'Corrigir a guia');
  await ok('31-faturamento-guia');
  await ir('/faturamento?aba=lotes');
  await page.getByRole('button', { name: /^Lote 148, Unimed/ }).click();
  await esperar(page, 'heading', 'Lote 148, Unimed');
  await ok('32-faturamento-lote');
  await ir('/faturamento?aba=glosas');
  await page.getByRole('button', { name: /^Sônia Maria Castro/ }).click();
  await esperar(page, 'form', 'Recurso da glosa');
  await ok('33-faturamento-recurso');
  await ir('/faturamento/auditoria');
  await page.getByRole('button', { name: /^Laudo não assinado/ }).click();
  await esperar(page, 'region', 'Guias com esta pendência');
  await ok('34-auditoria-faturamento');
  estado.sessao = SESSOES.recepcao;
  await ir('/autorizacoes');
  await page.getByRole('button', { name: /^Sônia Maria Castro/ }).click();
  await esperar(page, 'form', 'Protocolo do portal');
  await ok('35-autorizacoes-portal');
  await page.getByRole('button', { name: /^Antônio Carlos Ribeiro/ }).click();
  await esperar(page, 'region', 'Andamento');
  await ok('36-autorizacoes-autorizada');
  estado.sessao = SESSOES.recepcao;
  await ir('/contingencia');
  await page.getByText('Nenhuma contingência aberta').waitFor();
  await ok('26-contingencia');
  estado.contingenciaAberta = true;
  await ir('/contingencia');
  await esperar(page, 'region', 'Contingência aberta');
  await page.getByRole('button', { name: 'Lançar ficha' }).click();
  await ok('27-contingencia-ficha');
  estado.contingenciaAberta = false;
};

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
    const estado = novoEstado();
    await simularNucleo(page, estado);
    const ir = async (caminho: string): Promise<void> => {
      await page.goto(new URL(caminho, base).toString());
    };
    const ok = (nome: string): Promise<void> => conferir(page, nome, largura, altura);
    await m0(page, estado, ir, ok);
    await fase1(page, estado, ir, ok);
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
console.log(
  `Aprovado: ${String(conferidas)} conferências (${String(telas.size)} telas em ${String(JANELAS.length)} janelas), sem rolagem de página nem lateral. Prints em ${pastaDePrints}`,
);
