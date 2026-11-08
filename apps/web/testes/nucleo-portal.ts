/** Núcleo simulado do convite e do login do portal. As regras reais: `crates/api/tests/portal_acesso.rs`. */
import { http, HttpResponse } from 'msw';
import { registrar, temPerfil, semPermissao } from './nucleo-fase1';
import { problema } from './problema';

export const TOKEN_VALIDO = 'token-de-convite-valido-0000000000000';
export const portalSimulado = { logado: false };

export const manipuladoresPortal = [
  http.get('*/v1/portal/sessao', () =>
    portalSimulado.logado ? HttpResponse.json({ primeiro_nome: 'Antônio' }) : problema(401, 'nao-autenticado', 'Entre no portal.'),
  ),
  http.post('*/v1/portal/sessao', async ({ request }) => {
    const corpo = (await registrar('portal-entrar', request)) as { login: string; senha: string };
    if (corpo.login === 'antonio.ribeiro' && corpo.senha === 'uma senha longa 2026') {
      portalSimulado.logado = true;
      return new HttpResponse(null, { status: 204 });
    }
    return problema(401, 'login-nao-confere', 'Login ou senha não conferem. Confira e tente de novo; depois de 5 erros o acesso espera 15 minutos.');
  }),
  http.post('*/v1/portal/sessao/encerramento', () => {
    portalSimulado.logado = false;
    return new HttpResponse(null, { status: 204 });
  }),
  http.get('*/v1/portal/convites/:token', ({ params }) => HttpResponse.json({ situacao: params.token === TOKEN_VALIDO ? 'valido' : 'indisponivel' })),
  http.post('*/v1/portal/convites/:token/aceite', async ({ request }) => {
    const corpo = (await registrar('portal-aceite', request)) as { nascimento: string };
    if (corpo.nascimento !== '1954-06-15') {
      return problema(422, 'nascimento-nao-confere', 'A data de nascimento não confere com o cadastro. Nada foi gravado. Restam 4 tentativas.');
    }
    portalSimulado.logado = true;
    return new HttpResponse(null, { status: 204 });
  }),
  http.post('*/v1/pacientes/:id/convites-portal', async ({ request }) => {
    await registrar('convidar-portal', request);
    if (!temPerfil('recepcao', 'gerente')) return semPermissao('convidar para o portal');
    return HttpResponse.json(
      { id: '00000027-0000-7000-8000-000000000001', codigo: 'CNV-2026-000003', caminho: `/portal/convite/${TOKEN_VALIDO}`, expira_em: '2026-11-09T12:00:00Z' },
      { status: 201 },
    );
  }),
];

/* Cadastro de paciente: Maria Helena Duarte 12/03/1968 já existe (regras reais em tests/cadastro.rs). */
export const manipuladoresCadastro = [
  http.get('*/v1/pacientes/possiveis-duplicados', ({ request }) => {
    const u = new URL(request.url);
    const parecido = u.searchParams.get('nome')?.toLowerCase().startsWith('maria helena') === true && u.searchParams.get('nascimento')?.startsWith('1968-03') === true;
    return HttpResponse.json(
      parecido
        ? [{ paciente: { id: '0000000b-0000-7000-8000-000000000002', nome: 'Maria Helena Duarte', nascimento: '1968-03-12' }, codigo: 'PAC-2026-000002', motivo: 'Mesmo nome e nascimento parecido (dia e mês trocados ou dígito diferente)', bloqueante: false }]
        : [],
    );
  }),
  http.post('*/v1/pacientes', async ({ request }) => {
    const corpo = (await registrar('cadastrar-paciente', request)) as { nome: string; nascimento: string };
    if (!temPerfil('recepcao', 'gerente')) return semPermissao('cadastrar paciente');
    return HttpResponse.json({ id: '0000000b-0000-7000-8000-000000000099', codigo: 'PAC-2026-000101', nome: corpo.nome, nascimento: corpo.nascimento }, { status: 201 });
  }),
];
