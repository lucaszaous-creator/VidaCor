import { screen, waitFor, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID, voltarDa } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { LAUDO_ANTONIO_ID, SESSOES } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = `/laudos/${LAUDO_ANTONIO_ID}`;
const TITULO_ASSINATURA = 'Assinar o laudo de Antônio Carlos Ribeiro (15/06/1954)?';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medico;
});

function laudoJaAssinado(): void {
  const v = fase1.laudo.versoes[0];
  if (v === undefined) throw new Error('sem versão');
  fase1.laudo = {
    ...fase1.laudo,
    situacao: 'assinado',
    publicado_no_portal: true,
    versoes: [{ ...v, situacao: 'assinada', assinada_por: 'Dr. Paulo Medeiros', assinada_em: '2026-11-06T12:04:00Z', hash_documento: 'sha256:11111111' }],
  };
}

async function assinar(usuario: UserEvent, pin = '1234'): Promise<HTMLElement> {
  await usuario.click(await screen.findByRole('button', { name: 'Assinar (ICP-Brasil)' }));
  const dialogo = await screen.findByRole('alertdialog', { name: TITULO_ASSINATURA });
  await usuario.type(within(dialogo).getByLabelText('PIN do certificado'), pin);
  await usuario.click(within(dialogo).getByRole('button', { name: 'Assinar agora' }));
  return dialogo;
}

describe('Laudo: rascunho, assinatura e versões (FC-LAU)', () => {
  it('rascunho com salvamento automático no servidor, fila do médico e barra do paciente', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    expect(await screen.findByRole('region', { name: 'Paciente em foco: Antônio Carlos Ribeiro, nascimento 15/06/1954' })).toBeInTheDocument();
    const fila = screen.getByRole('region', { name: 'Minha fila de laudos' });
    expect(await within(fila).findByText('Conversar antes de liberar')).toBeInTheDocument();
    const texto = screen.getByRole('textbox', { name: 'Texto do laudo, versão 1' });
    expect(texto).toHaveValue('Ventrículo esquerdo com dimensões normais.\nFração de ejeção de 66% (Teichholz).\nÁtrio esquerdo de 36 mm.');
    await usuario.type(texto, '\nSem derrame pericárdico.');
    expect(await screen.findByText(/Rascunho salvo às/)).toBeInTheDocument();
    const ultimo = pedidosDe('rascunho').at(-1);
    expect(ultimo?.corpo).toMatchObject({ versao_numero: 1, texto: expect.stringContaining('Sem derrame pericárdico.') as unknown });
    expect(ultimo?.csrf).toBe('csrf-ficticio-dos-testes');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('assina com a confirmação nominal exata e o PIN; depois o texto fica só leitura e vai ao portal', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: 'Assinar (ICP-Brasil)' }));
    const dialogo = await screen.findByRole('alertdialog', { name: TITULO_ASSINATURA });
    await usuario.click(within(dialogo).getByRole('button', { name: 'Assinar agora' }));
    expect(within(dialogo).getByText('Digite o PIN do certificado.')).toBeInTheDocument();
    expect(pedidosDe('assinatura')).toHaveLength(0);
    expect(await violacoesDeAcessibilidade(dialogo)).toEqual([]);
    await usuario.type(within(dialogo).getByLabelText('PIN do certificado'), '1234{Enter}');

    expect(await screen.findByText(/Laudo assinado, versão 1\./)).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: /Texto do laudo/ })).not.toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Texto assinado, versão 1' })).toHaveTextContent('Átrio esquerdo de 36 mm.');
    expect(screen.getByText(/Publicado no portal do paciente\./)).toBeInTheDocument();
    expect(pedidosDe('assinatura')[0]?.chave).toMatch(UUID);
    expect(pedidosDe('assinatura')[0]?.corpo).toEqual({ pin: '1234' });
  });

  it('assinatura pede reautenticação: volta com a MESMA chave, o texto continua no rascunho do servidor', async () => {
    fase1.exigirReautenticacao.add('assinatura');
    const usuario = userEvent.setup();
    const primeira = renderizar(URL);
    const dialogo = await assinar(usuario);
    expect(await within(dialogo).findByText(/Confirme sua senha e o segundo fator para assinar o laudo\./)).toBeInTheDocument();
    expect(dialogo).toHaveTextContent('Por segurança, o PIN do certificado não fica guardado fora desta tela');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Confirmar identidade' }));
    const voltar = voltarDa(String(primeira.irPara.mock.calls[0]?.[0]));
    const chave = pedidosDe('assinatura')[0]?.chave ?? '';
    expect(voltar).toBe(`${URL}?retomar=assinar&chave=${chave}`);
    primeira.unmount();

    fase1.exigirReautenticacao.delete('assinatura');
    renderizar(voltar);
    const reaberto = await screen.findByRole('alertdialog', { name: TITULO_ASSINATURA });
    expect(reaberto).toHaveTextContent('O texto do laudo está salvo. Digite o PIN do certificado de novo para assinar.');
    await usuario.type(within(reaberto).getByLabelText('PIN do certificado'), '1234{Enter}');
    expect(await screen.findByText(/Laudo assinado, versão 1\./)).toBeInTheDocument();
    expect(pedidosDe('assinatura')[1]?.chave).toBe(chave);
  });

  it.each([
    [403, 'sem-permissao', 'Só o médico atribuído assina este laudo. Nada foi assinado.'],
    [409, 'assinatura-concorrente', 'O laudo acabou de ser assinado em outra estação. Nada foi assinado de novo.'],
  ])('assinatura com erro %i %s mostra a mensagem do servidor', async (status, codigo, detalhe) => {
    servidor.use(http.post('*/v1/laudos/:id/assinatura', () => problema(status, codigo, detalhe)));
    const usuario = userEvent.setup();
    renderizar(URL);
    const dialogo = await assinar(usuario);
    await esperarAlertaDoServidor(detalhe, codigo, dialogo);
  });

  it('rascunho recusado (409 laudo-assinado) avisa que não salvou e mantém o texto', async () => {
    servidor.use(
      http.put('*/v1/laudos/:id/rascunho', () =>
        problema(409, 'laudo-assinado', 'O laudo já está assinado e não pode ser alterado. Nada foi salvo.'),
      ),
    );
    const usuario = userEvent.setup();
    renderizar(URL);
    const texto = await screen.findByRole('textbox', { name: 'Texto do laudo, versão 1' });
    await usuario.type(texto, ' X');
    const aviso = await screen.findByText(/O rascunho não foi salvo\./);
    expect(aviso.closest('[role="status"]')).toHaveTextContent('O laudo já está assinado e não pode ser alterado. Nada foi salvo.');
    expect((texto as HTMLTextAreaElement).value).toContain('36 mm. X');
  });

  it('retificação exige motivo, cria a versão 2 e a 1 fica substituída depois de assinar', async () => {
    laudoJaAssinado();
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: 'Retificar' }));
    const dialogo = await screen.findByRole('dialog', { name: 'Retificar o laudo de Antônio Carlos Ribeiro?' });
    await usuario.click(within(dialogo).getByRole('button', { name: 'Criar retificação' }));
    expect(within(dialogo).getByText('Escreva o motivo da retificação.')).toBeInTheDocument();
    await usuario.type(within(dialogo).getByRole('textbox', { name: 'Motivo da retificação' }), 'medida do átrio esquerdo digitada errada');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Criar retificação' }));

    const editor = await screen.findByRole('textbox', { name: 'Texto do laudo, versão 2' });
    expect(screen.getByText(/Retificação, versão 2\./).closest('[role="status"]')).toHaveTextContent(
      'Motivo: medida do átrio esquerdo digitada errada.',
    );
    await usuario.clear(editor);
    await usuario.type(editor, 'Átrio esquerdo de 38 mm.');
    await assinar(usuario);
    expect(await screen.findByText(/Laudo assinado, versão 2\./)).toBeInTheDocument();
    const versoes = screen.getAllByRole('group').filter((g) => g.tagName === 'DETAILS');
    expect(versoes[0]).toHaveTextContent('Versão 2Vigente');
    expect(versoes[1]).toHaveTextContent('Versão 1Substituída');
    expect(pedidosDe('retificacao')[0]?.corpo).toMatchObject({ motivo: 'medida do átrio esquerdo digitada errada' });
  });

  it('retificação recusada pelo servidor (422 motivo-obrigatorio) mostra a mensagem', async () => {
    laudoJaAssinado();
    servidor.use(http.post('*/v1/laudos/:id/retificacao', () => problema(422, 'motivo-obrigatorio', 'A retificação precisa de motivo. Nada foi criado.')));
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: 'Retificar' }));
    const dialogo = await screen.findByRole('dialog');
    await usuario.type(within(dialogo).getByRole('textbox', { name: 'Motivo da retificação' }), '   x');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Criar retificação' }));
    await esperarAlertaDoServidor('A retificação precisa de motivo. Nada foi criado.', 'motivo-obrigatorio', dialogo);
  });

  it('"conversar antes" segura o portal; a liberação pede confirmação nominal depois da conversa', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.click(await screen.findByRole('checkbox', { name: /Conversar com o paciente antes de liberar/ }));
    await waitFor(() => {
      expect(pedidosDe('conversar-antes')[0]?.corpo).toEqual({ marcado: true });
    });
    await assinar(usuario);
    const retido = await screen.findByText(/Retido: conversar antes de liberar\./);
    expect(retido.closest('[role="status"]')).toHaveTextContent('O laudo não foi ao portal.');
    await usuario.click(screen.getByRole('button', { name: 'Liberar no portal após a conversa' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Liberar o laudo de Antônio Carlos Ribeiro no portal?' });
    expect(dialogo).toHaveTextContent('Antônio Carlos Ribeiro, nascimento 15/06/1954');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Liberar no portal' }));
    expect(await screen.findByText(/Publicado no portal do paciente\./)).toBeInTheDocument();
    expect(pedidosDe('liberacao')[0]?.chave).toMatch(UUID);
  });

  it('liberar rascunho no portal é recusado (409 laudo-nao-assinado)', async () => {
    laudoJaAssinado();
    fase1.laudo = { ...fase1.laudo, publicado_no_portal: false, conversar_antes: true };
    servidor.use(http.post('*/v1/laudos/:id/liberacao', () => problema(409, 'laudo-nao-assinado', 'O laudo ainda não foi assinado e não vai ao portal. Nada foi publicado.')));
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: 'Liberar no portal após a conversa' }));
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Liberar no portal' }));
    await esperarAlertaDoServidor('O laudo ainda não foi assinado e não vai ao portal.', 'laudo-nao-assinado');
  });

  it('texto do laudo aparece como texto, nunca como HTML', async () => {
    laudoJaAssinado();
    const v = fase1.laudo.versoes[0];
    if (v !== undefined) fase1.laudo = { ...fase1.laudo, versoes: [{ ...v, texto: '<b>negrito</b><img src=x onerror=alert(1)>' }] };
    renderizar(URL);
    const regiao = await screen.findByRole('region', { name: 'Texto assinado, versão 1' });
    expect(regiao).toHaveTextContent('<b>negrito</b><img src=x onerror=alert(1)>');
    expect(regiao.querySelector('b, img')).toBeNull();
  });
});

describe('Laudo: acesso indevido (FC-ACE)', () => {
  it('médico sem vínculo vê 403 e o acesso de emergência com justificativa mínima e reautenticação', async () => {
    estado.sessao = SESSOES.medicoSemVinculo;
    fase1.exigirReautenticacao.add('emergencia');
    const usuario = userEvent.setup();
    const primeira = renderizar(URL);
    expect(await screen.findByText('Você não tem vínculo de cuidado com este paciente')).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: /Paciente em foco/ })).not.toBeInTheDocument();
    const justificativa = screen.getByRole('textbox', { name: 'Justificativa' });
    await usuario.type(justificativa, 'urgência');
    await usuario.click(screen.getByRole('button', { name: 'Usar acesso de emergência' }));
    expect(screen.getByText('Escreva pelo menos 20 caracteres explicando a emergência.')).toBeInTheDocument();
    expect(pedidosDe('emergencia')).toHaveLength(0);

    await usuario.clear(justificativa);
    await usuario.type(justificativa, 'paciente em atendimento de urgência no pronto-socorro');
    await usuario.click(screen.getByRole('button', { name: 'Usar acesso de emergência' }));
    await usuario.click(await screen.findByRole('button', { name: 'Confirmar identidade' }));
    const voltar = voltarDa(String(primeira.irPara.mock.calls[0]?.[0]));
    expect(voltar).not.toContain('pronto-socorro');
    primeira.unmount();

    fase1.exigirReautenticacao.delete('emergencia');
    renderizar(voltar);
    expect(await screen.findByText(/Identidade confirmada\./)).toBeInTheDocument();
    await usuario.type(screen.getByRole('textbox', { name: 'Justificativa' }), 'paciente em atendimento de urgência no pronto-socorro');
    await usuario.click(screen.getByRole('button', { name: 'Usar acesso de emergência' }));
    expect(await screen.findByRole('region', { name: 'Paciente em foco: Antônio Carlos Ribeiro, nascimento 15/06/1954' })).toBeInTheDocument();
    expect(pedidosDe('emergencia')[1]?.chave).toBe(pedidosDe('emergencia')[0]?.chave);
  });

  it.each([
    ['recepção', SESSOES.recepcao],
    ['TI', SESSOES.ti],
  ])('%s não lê o conteúdo do laudo (403)', async (_nome, sessao) => {
    estado.sessao = sessao;
    renderizar(URL);
    expect(await screen.findByText('Seu perfil não lê laudos')).toBeInTheDocument();
    expect(screen.queryByText(/Átrio esquerdo/)).not.toBeInTheDocument();
  });
});
