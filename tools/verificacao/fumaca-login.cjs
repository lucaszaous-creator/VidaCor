// Fumaça do login real (dev): Keycloak com senha e 2FA (TOTP), sessão do VidaCor, agenda e
// check-in de Antônio Carlos Ribeiro. Uso: node tools/verificacao/fumaca-login.cjs [login]
// Pré-requisitos: `docker compose -f infra/compose/dev.yml up -d`, semente de dev, núcleo e tela
// no ar (http://127.0.0.1:5173). Só dados fictícios; senha de dev do realm de desenvolvimento.
// Observação: o 2FA é configurado no primeiro login; para repetir, recrie o realm de dev.
const { chromium } = require('playwright');
const crypto = require('crypto');
function totp(b32) {
  const alf = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'; let bits = '';
  for (const c of b32.replace(/\s|=/g,'').toUpperCase()) bits += alf.indexOf(c).toString(2).padStart(5,'0');
  const bytes = Buffer.from(bits.match(/.{8}/g).map(b => parseInt(b,2)));
  const cont = Buffer.alloc(8); cont.writeBigUInt64BE(BigInt(Math.floor(Date.now()/30000)));
  const h = crypto.createHmac('sha1', bytes).update(cont).digest(); const o = h[19] & 15;
  return String(((h.readUInt32BE(o) & 0x7fffffff) % 1e6)).padStart(6,'0');
}
const USUARIO = process.argv[2] || 'ana.souza';
const BASE = process.env.VIDACOR_URL || BASE + '';
(async () => {
  const b = await chromium.launch(); const page = await b.newPage();
  await page.goto(BASE + '/v1/sessao/entrar');
  await page.fill('#username', USUARIO); await page.fill('#password', 'VidaCor-dev-2026'); await page.click('#kc-login');
  await page.waitForLoadState();
  if (await page.locator('#mode-manual, a:has-text("Unable to scan")').count()) await page.locator('#mode-manual, a:has-text("Unable to scan")').first().click();
  const chave = (await page.locator('#kc-totp-secret-key').innerText()).trim();
  await page.fill('#totp', totp(chave)); await page.fill('#userLabel', 'teste'); await page.click('#saveTOTPBtn, input[type=submit]');
  await page.waitForURL(BASE + '/**');
  const sessao = await (await page.request.get(BASE + '/v1/sessao')).json();
  console.log('sessao:', sessao.usuario.nome, sessao.usuario.perfis, sessao.unidade.nome);
  const hoje = new Date().toLocaleDateString('sv-SE', {timeZone:'America/Sao_Paulo'});
  const agenda = await (await page.request.get(BASE + '/v1/agenda?data=' + hoje)).json();
  console.log('agenda:', agenda.itens.map(i => `${i.sala.nome} ${i.paciente.nome} ${i.paciente.nascimento} ${i.situacao}`).join(' | '));
  const ag = agenda.itens.find(i => i.paciente.nome === 'Antônio Carlos Ribeiro');
  const r = await page.request.post(BASE + '/v1/atendimentos', { headers: {'x-csrf-token': sessao.csrf, 'idempotency-key': crypto.randomUUID()},
    data: { agendamento_id: ag.id, confirmacao: { nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' } } });
  console.log('checkin:', r.status(), JSON.stringify(await r.json()));
  const cookies = await page.context().cookies(); console.log('cookie:', cookies.filter(c=>c.name.startsWith('vc_')).map(c=>`${c.name} httpOnly=${c.httpOnly} sameSite=${c.sameSite}`).join(', '));
  await b.close();
})().catch(e => { console.error('FALHOU', e.message); process.exit(1); });
