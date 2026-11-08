import { screen, waitFor, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID, voltarDa } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { caixaAberto, SESSOES, SONIA } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = '/caixa?data=2026-11-06';
const AUTORIZACAO = '00000021-0000-7000-8000-000000000001';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.caixa;
});

async function preencher(usuario: UserEvent, opcoes: { percentual?: string; forma?: string } = {}): Promise<void> {
  const paciente = await screen.findByRole('combobox', { name: 'Paciente (check-in feito hoje)' });
  await screen.findByRole('option', { name: /^Sônia Maria Castro, 03\/02\/1962/ });
  await usuario.selectOptions(paciente, SONIA.atendimento_id ?? '');
  await usuario.type(screen.getByRole('textbox', { name: 'Procedimentos (códigos TUSS)' }), '40901114');
  if (opcoes.percentual !== undefined) {
    await usuario.type(screen.getByRole('textbox', { name: 'Desconto (%)' }), opcoes.percentual);
    await usuario.selectOptions(screen.getByRole('combobox', { name: 'Motivo do desconto' }), 'Tabela social');
  }
  await usuario.selectOptions(screen.getByRole('combobox', { name: 'Forma de pagamento' }), opcoes.forma ?? 'pix');
}

describe('Caixa do particular (FC-CXA, FC-PIX)', () => {
  it('sem caixa aberto, a tela leva a abrir com o fundo de troco', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    expect(await screen.findByText(/Não há caixa aberto em seu nome\./)).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.type(screen.getByRole('textbox', { name: 'Fundo de troco (R$)' }), '200,00');
    await usuario.click(screen.getByRole('button', { name: 'Abrir caixa' }));
    expect(await screen.findByRole('heading', { name: 'Caixa CX-2026-000312' })).toBeInTheDocument();
    expect(pedidosDe('abrir-caixa')[0]?.corpo).toEqual({ fundo_centavos: 20000 });
    expect(pedidosDe('abrir-caixa')[0]?.chave).toMatch(UUID);
  });

  it('o total vem do servidor com a versão da tabela; a tela não manda total e mostra o PIX copia e cola', async () => {
    fase1.caixa = caixaAberto();
    fase1.pspRespondeu = false;
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    await preencher(usuario);
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    const tabela = await screen.findByRole('table', { name: 'Itens calculados pelo servidor' });
    expect(tabela).toHaveTextContent('Ecocardiograma transtorácico TUSS 40901114R$ 380,00');
    expect(tabela).toHaveTextContent('TotalR$ 380,00');
    expect(screen.getByText(/pela tabela/)).toHaveTextContent('Valores calculados pelo servidor pela tabela versão 12.');
    expect(screen.getByText('Aguardando pagamento')).toBeInTheDocument();
    // O PSP devolve a cobrança pelo outbox: a tela avisa e consulta o recebimento até o PIX chegar.
    expect(await screen.findByText(/Gerando a cobrança PIX\./)).toBeInTheDocument();
    await waitFor(() => {
      expect(fase1.recebimento?.consultas).toBeGreaterThan(0);
    });
    fase1.pspRespondeu = true;
    expect(await screen.findByText('00020126580014br.gov.bcb.pix0136vidacor-ficticio-4410')).toBeInTheDocument();
    expect(screen.queryByText(/Gerando a cobrança PIX\./)).not.toBeInTheDocument();
    expect(pedidosDe('recebimento')).toHaveLength(1);
    expect(pedidosDe('recebimento')[0]?.corpo).toEqual({
      paciente_id: SONIA.paciente.id,
      atendimento_id: SONIA.atendimento_id,
      itens: [{ tuss: '40901114' }],
      desconto: null,
      forma: 'pix',
    });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('desconto dentro do limite: o servidor devolve o total com desconto', async () => {
    fase1.caixa = caixaAberto();
    const usuario = userEvent.setup();
    renderizar(URL);
    await preencher(usuario, { percentual: '5', forma: 'dinheiro' });
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    const tabela = await screen.findByRole('table');
    expect(tabela).toHaveTextContent('DescontoR$ 19,00');
    expect(tabela).toHaveTextContent('TotalR$ 361,00');
    expect(pedidosDe('recebimento')[0]?.corpo).toMatchObject({ desconto: { pontos_base: 500, motivo: 'Tabela social' } });
  });

  it('desconto acima do limite (403): oferece pedir autorização e reenvia com o código da gerência', async () => {
    fase1.caixa = caixaAberto();
    const usuario = userEvent.setup();
    renderizar(URL);
    await preencher(usuario, { percentual: '15' });
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    const alerta = await esperarAlertaDoServidor('O desconto de 15,00% passa do seu limite de 5,00%.', 'desconto-acima-do-limite');
    expect(alerta).toHaveTextContent('Seu limite é 5,00%.');
    await usuario.click(within(alerta).getByRole('button', { name: 'Pedir autorização à gerência' }));
    const pedido = screen.getByText(/Pedido de autorização\./).closest('[role="status"]');
    expect(pedido).toHaveTextContent(`/caixa/autorizacao-desconto?paciente=${SONIA.paciente.id}&pontos=1500&solicitante=${SESSOES.caixa.usuario.id}`);
    await usuario.type(screen.getByRole('textbox', { name: 'Código da autorização da gerência' }), AUTORIZACAO);
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    expect(await screen.findByRole('table')).toHaveTextContent('TotalR$ 323,00');
    const [primeiro, segundo] = pedidosDe('recebimento');
    expect(segundo?.corpo).toMatchObject({ desconto: { pontos_base: 1500, motivo: 'Tabela social', autorizacao_id: AUTORIZACAO } });
    expect(segundo?.chave).not.toBe(primeiro?.chave);
  });

  it('caixa fechado (409) orienta abrir o caixa', async () => {
    fase1.caixa = caixaAberto();
    servidor.use(
      http.post('*/v1/recebimentos', () => {
        fase1.caixa = null;
        return problema(409, 'caixa-fechado', 'Não há caixa aberto em seu nome. Nada foi recebido. Abra o caixa e repita o recebimento.');
      }),
    );
    const usuario = userEvent.setup();
    renderizar(URL);
    await preencher(usuario);
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    const alerta = await esperarAlertaDoServidor('Não há caixa aberto em seu nome. Nada foi recebido.', 'caixa-fechado');
    await usuario.click(within(alerta).getByRole('button', { name: 'Abrir o caixa' }));
    expect(await screen.findByRole('textbox', { name: 'Fundo de troco (R$)' })).toBeInTheDocument();
  });

  it.each([
    [422, 'valor-invalido', 'Um item tem valor zero ou negativo. Nada foi recebido.'],
    [409, 'sem-tabela-de-precos', 'Não há tabela de preços vigente hoje. Nada foi recebido. Avise a coordenação.'],
  ])('recebimento com erro %i %s mostra a mensagem do servidor', async (status, codigo, detalhe) => {
    fase1.caixa = caixaAberto();
    servidor.use(http.post('*/v1/recebimentos', () => problema(status, codigo, detalhe)));
    const usuario = userEvent.setup();
    renderizar(URL);
    await preencher(usuario);
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    await esperarAlertaDoServidor(detalhe, codigo);
  });

  it('quem pede o estorno não vê o botão de aprovar', async () => {
    fase1.caixa = caixaAberto();
    const usuario = userEvent.setup();
    renderizar(URL);
    await preencher(usuario);
    await usuario.click(screen.getByRole('button', { name: 'Registrar recebimento' }));
    await usuario.click(await screen.findByRole('button', { name: 'Pedir estorno' }));
    const dialogo = await screen.findByRole('dialog', { name: 'Pedir o estorno do recebimento REC-2026-004410?' });
    await usuario.type(within(dialogo).getByRole('textbox', { name: 'Motivo do estorno' }), 'cobrança em duplicidade');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Pedir estorno' }));
    const feito = await screen.findByText(/Estorno pedido\./);
    expect(feito.closest('[role="status"]')).toHaveTextContent('quem pediu não aprova o próprio pedido');
    expect(screen.queryByRole('button', { name: /Aprovar/ })).not.toBeInTheDocument();
  });

  it('fechamento mostra esperado, contado e a diferença sem esconder', async () => {
    fase1.caixa = caixaAberto();
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: 'Fechar caixa' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Fechar o caixa CX-2026-000312?' });
    await usuario.type(within(dialogo).getByRole('textbox', { name: 'Total contado na gaveta (R$)' }), '700,00{Enter}');
    const fechado = await screen.findByText(/Caixa fechado\./);
    expect(fechado.closest('[role="status"]')).toHaveTextContent('Falta de R$ 40,00.');
    expect(screen.getByText('R$ 740,00')).toBeInTheDocument();
    expect(screen.getByText('R$ 700,00')).toBeInTheDocument();
    expect(screen.getByText('-R$ 40,00')).toBeInTheDocument();
    expect(pedidosDe('fechamento')[0]?.corpo).toEqual({ contado_centavos: 70000 });
  });
});

describe('Gerência: autorização de desconto e aprovação de estorno', () => {
  it('autoriza o desconto conferindo o paciente, com reautenticação e a mesma chave na volta', async () => {
    estado.sessao = SESSOES.gerente;
    fase1.exigirReautenticacao.add('autorizacao');
    const usuario = userEvent.setup();
    const url = `/caixa/autorizacao-desconto?paciente=${SONIA.paciente.id}&pontos=1500&solicitante=${SESSOES.caixa.usuario.id}`;
    const primeira = renderizar(url);
    const percentual = await screen.findByRole('textbox', { name: 'Desconto autorizado (%)' });
    expect(percentual).toHaveValue('15,00');
    await usuario.type(screen.getByRole('searchbox', { name: 'Buscar paciente por nome' }), 'Sônia');
    await usuario.click(await screen.findByRole('button', { name: 'Sônia Maria Castro, nascimento 03/02/1962' }));
    await usuario.selectOptions(screen.getByRole('combobox', { name: 'Motivo' }), 'Tabela social');
    await usuario.click(screen.getByRole('button', { name: 'Autorizar desconto' }));
    await usuario.click(await screen.findByRole('button', { name: 'Confirmar identidade' }));
    const voltar = voltarDa(String(primeira.irPara.mock.calls[0]?.[0]));
    const chave = pedidosDe('autorizacao')[0]?.chave ?? '';
    expect(voltar).toContain(`paciente=${SONIA.paciente.id}`);
    expect(voltar).toContain(`chave=${chave}`);
    expect(voltar).not.toContain('Sônia');
    primeira.unmount();

    fase1.exigirReautenticacao.delete('autorizacao');
    renderizar(voltar);
    expect(await screen.findByText(/Identidade confirmada\./)).toBeInTheDocument();
    await usuario.type(screen.getByRole('searchbox', { name: 'Buscar paciente por nome' }), 'Sônia');
    await usuario.click(await screen.findByRole('button', { name: 'Sônia Maria Castro, nascimento 03/02/1962' }));
    await usuario.selectOptions(screen.getByRole('combobox', { name: 'Motivo' }), 'Tabela social');
    await usuario.click(screen.getByRole('button', { name: 'Autorizar desconto' }));
    const feito = await screen.findByText(/Desconto autorizado\./);
    expect(feito.closest('[role="status"]')).toHaveTextContent(AUTORIZACAO);
    expect(pedidosDe('autorizacao')[1]?.chave).toBe(chave);
    expect(pedidosDe('autorizacao')[1]?.corpo).toEqual({
      paciente_id: SONIA.paciente.id,
      pontos_base: 1500,
      motivo: 'Tabela social',
      solicitante_id: SESSOES.caixa.usuario.id,
    });
  });

  it('avisa quando o paciente escolhido não é o do pedido do caixa', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    renderizar(`/caixa/autorizacao-desconto?paciente=${SONIA.paciente.id}&pontos=1500&solicitante=${SESSOES.caixa.usuario.id}`);
    await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar paciente por nome' }), 'Vera');
    await usuario.click(await screen.findByRole('button', { name: 'Vera Lúcia Antunes, nascimento 18/09/1957' }));
    expect(screen.getByText(/Este não é o paciente do pedido do caixa\./)).toBeInTheDocument();
  });

  it('gerente aprova o estorno pedido por outra pessoa', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    const { container } = renderizar('/caixa/estornos?estorno=00000022-0000-7000-8000-000000000001');
    await usuario.click(await screen.findByRole('button', { name: 'Revisar e aprovar' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Aprovar este estorno?' });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(dialogo).getByRole('button', { name: 'Aprovar estorno' }));
    expect(await screen.findByText(/Estorno aprovado\./)).toBeInTheDocument();
  });

  it('quem pediu o estorno recebe 403 separacao-de-funcoes ao tentar aprovar', async () => {
    estado.sessao = SESSOES.caixa;
    const usuario = userEvent.setup();
    renderizar('/caixa/estornos?estorno=00000022-0000-7000-8000-000000000001');
    await usuario.click(await screen.findByRole('button', { name: 'Revisar e aprovar' }));
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Aprovar estorno' }));
    await esperarAlertaDoServidor('Quem pediu o estorno não pode aprovar o próprio pedido.', 'separacao-de-funcoes');
  });

  it('aprovação pede reautenticação e volta com a mesma chave', async () => {
    estado.sessao = SESSOES.gerente;
    fase1.exigirReautenticacao.add('aprovar-estorno');
    const usuario = userEvent.setup();
    const primeira = renderizar('/caixa/estornos?estorno=00000022-0000-7000-8000-000000000001');
    await usuario.click(await screen.findByRole('button', { name: 'Revisar e aprovar' }));
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Aprovar estorno' }));
    await usuario.click(await screen.findByRole('button', { name: 'Confirmar identidade' }));
    const voltar = voltarDa(String(primeira.irPara.mock.calls[0]?.[0]));
    primeira.unmount();
    fase1.exigirReautenticacao.delete('aprovar-estorno');
    renderizar(voltar);
    await usuario.click(within(await screen.findByRole('alertdialog', { name: 'Aprovar este estorno?' })).getByRole('button', { name: 'Aprovar estorno' }));
    expect(await screen.findByText(/Estorno aprovado\./)).toBeInTheDocument();
    const [a, b] = pedidosDe('aprovar-estorno');
    expect(b?.chave).toBe(a?.chave);
  });

  it('caixa sem permissão vê o estado de sem permissão', async () => {
    servidor.use(http.get('*/v1/caixas/meu', () => problema(403, 'sem-permissao', 'Seu perfil não opera caixa.')));
    estado.sessao = SESSOES.ti;
    renderizar(URL);
    expect(await screen.findByText('Você não opera caixa')).toBeInTheDocument();
  });
});
