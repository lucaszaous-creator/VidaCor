// Fumaça do convite do portal contra o NÚCLEO REAL (dev): a recepção (Keycloak com TOTP) convida
// Antônio; o paciente abre o link em outro navegador, confirma o nascimento, cria a senha e vê os
// resultados; depois sai e entra de novo pelo login. Só dados fictícios.
// Uso: node tools/verificacao/fumaca-portal.cjs   (realm de dev recém-criado: o 2FA é configurado agora)
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
(async () => {
  const b = await chromium.launch();
  const equipe = await (await b.newContext()).newPage();
  await equipe.goto(BASE + '/v1/sessao/entrar');
  await equipe.fill('#username', 'ana.souza'); await equipe.fill('#password', 'VidaCor-dev-2026'); await equipe.click('#kc-login');
  await equipe.waitForLoadState();
  const manual = equipe.locator('#mode-manual, a:has-text("Unable to scan")'); if (await manual.count()) await manual.first().click();
  const chave = (await equipe.locator('#kc-totp-secret-key').innerText()).trim();
  await equipe.fill('#totp', totp(chave)); await equipe.fill('#userLabel', 'fumaca'); await equipe.click('#saveTOTPBtn, input[type=submit]');
  await equipe.waitForURL(BASE + '/**');
  const sessao = await (await equipe.request.get(BASE + '/v1/sessao')).json();
  const pacientes = await (await equipe.request.get(BASE + '/v1/pacientes?busca=' + encodeURIComponent('Antônio'))).json();
  const antonio = pacientes.find(p => p.nascimento === '1954-06-15');
  const r = await equipe.request.post(`${BASE}/v1/pacientes/${antonio.id}/convites-portal`, { headers: { 'x-csrf-token': sessao.csrf, 'idempotency-key': crypto.randomUUID() }, data: {} });
  const convite = await r.json();
  console.log('convite:', r.status(), convite.codigo, convite.caminho ? 'link recebido' : 'SEM LINK');
  const paciente = await (await b.newContext({ viewport: { width: 1280, height: 800 } })).newPage();
  await paciente.goto(BASE + convite.caminho);
  await paciente.getByRole('heading', { name: 'Primeiro acesso' }).waitFor();
  await paciente.getByRole('spinbutton').first().click(); await paciente.keyboard.type('15061954');
  const login = 'antonio.' + Date.now().toString(36);
  await paciente.getByRole('textbox', { name: 'Login' }).fill(login);
  await paciente.getByLabel('Senha', { exact: true }).fill('uma senha longa 2026');
  await paciente.getByLabel('Repita a senha').fill('uma senha longa 2026');
  await paciente.getByRole('button', { name: 'Criar acesso e entrar' }).click();
  await paciente.getByRole('heading', { name: 'Seus resultados' }).waitFor();
  console.log('aceite: ok,', await paciente.getByText(/^Olá,/).innerText());
  const cookie = (await paciente.context().cookies()).find(c => c.name === 'vc_portal');
  console.log('cookie do portal:', cookie ? `httpOnly=${cookie.httpOnly} sameSite=${cookie.sameSite}` : 'AUSENTE');
  await paciente.getByRole('button', { name: 'Sair' }).click();
  await paciente.getByRole('heading', { name: 'Entrar' }).waitFor();
  await paciente.goto(BASE + convite.caminho);
  await paciente.getByText('Este convite não vale mais.').waitFor();
  console.log('uso único: ok');
  await paciente.goto(BASE + '/portal/entrar');
  await paciente.getByRole('textbox', { name: 'Login' }).fill(login);
  await paciente.getByLabel('Senha', { exact: true }).fill('uma senha longa 2026');
  await paciente.getByRole('button', { name: 'Entrar' }).click();
  await paciente.getByRole('heading', { name: 'Seus resultados' }).waitFor();
  console.log('login de novo: ok');
  await b.close();
})().catch(e => { console.error('FALHOU', e.message); process.exit(1); });
