// Fumaça das telas da Fase 1 contra o NÚCLEO REAL (dev): para cada perfil, entra pelo Keycloak (senha
// + TOTP), abre as telas do perfil em 1440×900 e 1280×800 e registra: respostas da API com erro,
// erros do console, rolagem da página e rolagem lateral. Não substitui os testes G4 nem o E2E.
// Uso: node tools/verificacao/fumaca-telas.cjs [pasta-de-capturas]
// Pré-requisitos: compose de dev, semente, núcleo com VIDACOR_ASSINADOR e VIDACOR_PSP=desenvolvimento,
// tela em http://127.0.0.1:5173 e realm de dev recém-criado (o 2FA é configurado no primeiro login).
const { chromium } = require('playwright');
const crypto = require('crypto');
function totp(b32) {
  const alf = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'; let bits = '';
  for (const c of b32.replace(/\s|=/g, '').toUpperCase()) bits += alf.indexOf(c).toString(2).padStart(5, '0');
  const bytes = Buffer.from(bits.match(/.{8}/g).map(b => parseInt(b, 2)));
  const cont = Buffer.alloc(8); cont.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30000)));
  const h = crypto.createHmac('sha1', bytes).update(cont).digest(); const o = h[19] & 15;
  return String(((h.readUInt32BE(o) & 0x7fffffff) % 1e6)).padStart(6, '0');
}
const BASE = process.env.VIDACOR_URL || 'http://127.0.0.1:5173';
const PASTA = process.argv[2];
const PERFIS = [
  ['ana.souza', ['/recepcao', '/agendamentos/novo', '/contingencia']],
  ['bruno.sales', ['/sala']],
  ['paulo.mendes', ['/laudos', '/estudos']],
  ['carla.caixa', ['/caixa']],
  ['patricia.lemos', ['/caixa/autorizacao-desconto', '/caixa/estornos', '/contingencia']],
];
async function entrar(page, usuario) {
  await page.goto(BASE + '/v1/sessao/entrar');
  await page.fill('#username', usuario); await page.fill('#password', 'VidaCor-dev-2026'); await page.click('#kc-login');
  await page.waitForLoadState();
  const manual = page.locator('#mode-manual, a:has-text("Unable to scan")');
  if (await manual.count()) await manual.first().click();
  const chave = (await page.locator('#kc-totp-secret-key').innerText()).trim();
  await page.fill('#totp', totp(chave)); await page.fill('#userLabel', 'fumaca'); await page.click('#saveTOTPBtn, input[type=submit]');
  await page.waitForURL(BASE + '/**');
}
(async () => {
  const b = await chromium.launch(); let falhas = 0;
  for (const [usuario, telas] of PERFIS) {
    const ctx = await b.newContext({ viewport: { width: 1440, height: 900 } }); const page = await ctx.newPage();
    const erros = [];
    page.on('console', m => { if (m.type() === 'error') erros.push('console: ' + m.text().slice(0, 160)); });
    page.on('response', r => { if (r.url().includes('/v1/') && r.status() >= 400 && !r.url().includes('/sessao')) erros.push(`api ${r.status()} ${r.request().method()} ${new URL(r.url()).pathname}`); });
    try { await entrar(page, usuario); } catch (e) { console.log(`${usuario}: LOGIN FALHOU ${e.message}`); falhas++; await ctx.close(); continue; }
    for (const tela of telas) {
      for (const [w, h] of [[1440, 900], [1280, 800]]) {
        erros.length = 0;
        await page.setViewportSize({ width: w, height: h });
        await page.goto(BASE + tela); await page.waitForLoadState('networkidle');
        const m = await page.evaluate(() => ({ titulo: document.querySelector('h1')?.textContent?.trim(),
          rolaV: document.documentElement.scrollHeight > innerHeight + 1, rolaH: document.documentElement.scrollWidth > innerWidth + 1,
          alerta: [...document.querySelectorAll('[role=alert]')].map(e => e.textContent.trim().slice(0, 120)) }));
        const ruim = m.rolaV || m.rolaH || erros.length;
        if (ruim) falhas++;
        console.log(`${ruim ? 'FALHA' : 'ok   '} ${usuario} ${tela} ${w}x${h} h1="${m.titulo}"${m.rolaV ? ' ROLA-PAGINA' : ''}${m.rolaH ? ' ROLA-LATERAL' : ''}${m.alerta.length ? ' alerta=' + JSON.stringify(m.alerta) : ''}${erros.length ? ' ' + JSON.stringify([...new Set(erros)]) : ''}`);
        if (PASTA && w === 1440) await page.screenshot({ path: `${PASTA}/${usuario}${tela.replace(/\//g, '_')}.png` });
      }
    }
    await ctx.close();
  }
  await b.close(); console.log(falhas ? `${falhas} falha(s)` : 'todas as telas ok'); process.exit(falhas ? 1 : 0);
})().catch(e => { console.error('FALHOU', e.message); process.exit(1); });
