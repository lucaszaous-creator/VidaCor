import { createHash } from 'node:crypto';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { contingenciaAberta, JULIANA, MARIA_HELENA, SESSOES, SONIA } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const CONTEUDO_PDF = '%PDF-1.4 ficha F-ATD ficticia';

beforeEach(() => {
  reiniciarNucleo();
});

interface Ficha {
  numero: string;
  agendamento: string;
  nome: string;
  nascimento: string;
}

const FICHA_MARIA: Ficha = { numero: '0412', agendamento: MARIA_HELENA.id, nome: 'Maria Helena Duarte', nascimento: '12031968' };

async function lancarFicha(usuario: UserEvent, f: Ficha): Promise<void> {
  const form = await screen.findByRole('form', { name: 'Ficha F-ATD' });
  await usuario.type(within(form).getByRole('textbox', { name: 'Número da ficha' }), f.numero);
  await digitarData(usuario, 'Dia do atendimento', '06112026', form);
  await usuario.type(within(form).getByRole('textbox', { name: 'Hora do atendimento' }), '10:50');
  const agendamento = within(form).getByRole('combobox', { name: 'Agendamento (código impresso na agenda do kit)' });
  await within(form).findByRole('option', { name: /^AG-2611-4100, 07:00, Maria Helena Duarte, 12\/03\/1968$/ });
  await usuario.selectOptions(agendamento, f.agendamento);
  await usuario.type(within(form).getByRole('textbox', { name: 'Nome como está no papel' }), f.nome);
  await digitarData(usuario, 'Nascimento como está no papel', f.nascimento, form);
  await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Quem atendeu' }), within(form).getByRole('option', { name: /^Eu, / }));
  await usuario.upload(within(form).getByLabelText('Ficha digitalizada (PDF)'), new File([CONTEUDO_PDF], `f-atd-${f.numero}.pdf`, { type: 'application/pdf' }));
  await within(form).findByText(/SHA-256/);
  await usuario.click(within(form).getByRole('button', { name: 'Lançar ficha' }));
}

describe('Contingência (FC-CTG)', () => {
  it('declara com a chave da estação e a hora original; a faixa aparece no topo', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/contingencia');
    expect(await screen.findByText('Nenhuma contingência aberta')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Declarar contingência' }));
    const dialogo = await screen.findByRole('dialog', { name: 'Declarar contingência?' });
    await usuario.click(within(dialogo).getByRole('button', { name: 'Declarar agora' }));
    expect(within(dialogo).getByText('Escolha o nível.')).toBeInTheDocument();
    await usuario.selectOptions(within(dialogo).getByRole('combobox', { name: 'Nível' }), 'total');
    await usuario.selectOptions(within(dialogo).getByRole('combobox', { name: 'Motivo' }), 'servidor local fora do ar');
    expect(dialogo).toHaveTextContent('Responsável: Ana Souza (você).');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Declarar agora' }));

    const painel = await screen.findByRole('region', { name: 'Contingência aberta' });
    expect(painel).toHaveTextContent('CTG-2026-0007');
    expect(await screen.findByText(/Contingência CTG-2026-0007 aberta/)).toBeInTheDocument();
    const pedido = pedidosDe('declarar')[0];
    const corpo = pedido?.corpo as { chave: string; declarada_em: string; responsavel_id: string; nivel: string };
    expect(corpo.chave).toMatch(UUID);
    expect(pedido?.chave).toBe(corpo.chave);
    expect(corpo.responsavel_id).toBe(SESSOES.recepcao.usuario.id);
    expect(corpo.nivel).toBe('total');
    expect(Number.isNaN(Date.parse(corpo.declarada_em))).toBe(false);
  });

  it('perfil sem permissão não declara (403)', async () => {
    estado.sessao = SESSOES.caixa;
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await usuario.click(await screen.findByRole('button', { name: 'Declarar contingência' }));
    const dialogo = await screen.findByRole('dialog');
    await usuario.selectOptions(within(dialogo).getByRole('combobox', { name: 'Nível' }), 'setor');
    await usuario.selectOptions(within(dialogo).getByRole('combobox', { name: 'Motivo' }), 'simulado');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Declarar agora' }));
    await esperarAlertaDoServidor('Seu perfil não pode declarar contingência.', 'sem-permissao', dialogo);
  });

  it('ficha F-ATD lançada leva a marca de contingência e o hash do PDF', async () => {
    fase1.contingencia = contingenciaAberta();
    const usuario = userEvent.setup();
    const { container } = renderizar('/contingencia');
    await lancarFicha(usuario, FICHA_MARIA);
    const lista = await screen.findByRole('list', { name: 'Lançamentos desta sessão' });
    expect(await within(lista).findByText(/Ficha F-ATD 0412: atendimento ATD-2026-119412 de Maria Helena Duarte\. Registrado em contingência\./)).toBeInTheDocument();
    const pedido = pedidosDe('ficha')[0];
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo).toMatchObject({
      formulario: 'F-ATD',
      numero: 412,
      agendamento_id: MARIA_HELENA.id,
      nome: 'Maria Helena Duarte',
      nascimento: '1968-03-12',
      atendido_por: SESSOES.recepcao.usuario.id,
      anexo_sha256: createHash('sha256').update(CONTEUDO_PDF).digest('hex'),
    });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('nascimento do papel diferente abre divergência (202), sem alterar cadastro', async () => {
    fase1.contingencia = contingenciaAberta();
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await lancarFicha(usuario, { numero: '0414', agendamento: JULIANA.id, nome: JULIANA.paciente.nome, nascimento: '12041992' });
    const lista = await screen.findByRole('list', { name: 'Lançamentos desta sessão' });
    expect(await within(lista).findByText(/Ficha F-ATD 0414: divergência DIV-2026-000031 aberta\./)).toHaveTextContent('Nenhum cadastro foi alterado');
  });

  it('divergência aberta pode ser resolvida pela gerência', async () => {
    fase1.contingencia = contingenciaAberta();
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await lancarFicha(usuario, { numero: '0414', agendamento: JULIANA.id, nome: JULIANA.paciente.nome, nascimento: '12041992' });
    await usuario.click(await screen.findByRole('button', { name: 'Resolver DIV-2026-000031' }));
    const dialogo = await screen.findByRole('dialog', { name: 'Resolver a divergência DIV-2026-000031?' });
    await usuario.type(within(dialogo).getByRole('textbox', { name: 'Resolução' }), 'papel com ano errado; cadastro conferido com o documento');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Registrar resolução' }));
    expect(await screen.findByText('Resolvida')).toBeInTheDocument();
    expect(pedidosDe('resolucao')[0]?.corpo).toEqual({ resolucao: 'papel com ano errado; cadastro conferido com o documento' });
  });

  it('números da faixa sem uso são registrados como devolvidos ou anulados', async () => {
    fase1.contingencia = contingenciaAberta();
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await usuario.click(await screen.findByRole('tab', { name: 'Números sem uso' }));
    const form = await screen.findByRole('form', { name: 'Números sem uso' });
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Formulário' }), 'F-ATD');
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Situação' }), 'devolvido');
    await usuario.type(within(form).getByRole('textbox', { name: 'Do número' }), '0480');
    await usuario.type(within(form).getByRole('textbox', { name: 'Até o número' }), '0499');
    await usuario.click(within(form).getByRole('button', { name: 'Registrar números' }));
    expect(await within(form).findByText(/Números registrados\./)).toBeInTheDocument();
    expect(pedidosDe('numeros')[0]?.corpo).toEqual({ formulario: 'F-ATD', de: 480, ate: 499, situacao: 'devolvido' });
    expect(pedidosDe('numeros')[0]?.chave).toMatch(UUID);
  });

  it.each([
    [409, 'ficha-ja-lancada', 'A ficha F-ATD 0412 já foi lançada no atendimento ATD-2026-119412. Nada foi lançado de novo.'],
    [422, 'numero-fora-da-faixa', 'A ficha F-ATD 0612 não está na faixa reservada para hoje (0400 a 0499). Nada foi lançado.'],
  ])('ficha com erro %i %s mostra a mensagem do servidor', async (status, codigo, detalhe) => {
    fase1.contingencia = contingenciaAberta();
    servidor.use(http.post('*/v1/contingencias/:id/fichas', () => problema(status, codigo, detalhe)));
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await lancarFicha(usuario, FICHA_MARIA);
    await esperarAlertaDoServidor(detalhe, codigo);
  });

  it('recibo F-REC com valor diferente da tabela grava o recalculado e abre divergência de preço', async () => {
    fase1.contingencia = contingenciaAberta();
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await usuario.click(await screen.findByRole('tab', { name: 'Recibo F-REC' }));
    const form = await screen.findByRole('form', { name: 'Recibo F-REC' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Número do recibo' }), '0207');
    await usuario.type(within(form).getByRole('searchbox', { name: 'Paciente do recibo' }), 'Sônia');
    await usuario.click(await within(form).findByRole('button', { name: /^Sônia Maria Castro, nascimento 03\/02\/1962/ }));
    await usuario.type(within(form).getByRole('textbox', { name: 'Procedimento (TUSS)' }), '40901114');
    await usuario.type(within(form).getByRole('textbox', { name: 'Valor escrito no papel (R$)' }), '300,00');
    await digitarData(usuario, 'Dia do recibo', '06112026', form);
    await usuario.type(within(form).getByRole('textbox', { name: 'Hora do recibo' }), '11:05');
    await usuario.click(within(form).getByRole('button', { name: 'Lançar recibo' }));
    const lista = await screen.findByRole('list', { name: 'Lançamentos desta sessão' });
    expect(await within(lista).findByText(/Recibo F-REC 0207: papel R\$ 300,00, tabela R\$ 380,00\. Divergência DIV-2026-000040 aberta/)).toBeInTheDocument();
    expect(pedidosDe('recibo')[0]?.corpo).toMatchObject({ numero: 207, paciente_id: SONIA.paciente.id, valor_papel_centavos: 30000 });
  });

  it('encerramento com pendência (409) lista o que falta; sem pendência, encerra', async () => {
    fase1.contingencia = contingenciaAberta();
    fase1.divergenciasAbertas.push('DIV-2026-000031');
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await usuario.click(await screen.findByRole('button', { name: 'Encerrar contingência' }));
    await usuario.click(within(await screen.findByRole('dialog', { name: 'Encerrar a contingência CTG-2026-0007?' })).getByRole('button', { name: 'Encerrar' }));
    const alerta = await esperarAlertaDoServidor('A contingência não pode ser encerrada com pendências.', 'pendencias');
    const pendencias = within(alerta).getByRole('list', { name: 'Pendências' });
    expect(within(pendencias).getAllByRole('listitem').map((i) => i.textContent)).toEqual([
      'Divergência DIV-2026-000031 aberta',
      'Ficha F-ATD 0413 sem situação',
    ]);

    fase1.divergenciasAbertas = [];
    await usuario.click(screen.getByRole('button', { name: 'Encerrar contingência' }));
    await usuario.click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Encerrar' }));
    await waitFor(() => {
      expect(screen.getByText('Nenhuma contingência aberta')).toBeInTheDocument();
    });
    const [a, b] = pedidosDe('encerrar');
    expect(b?.chave).not.toBe(a?.chave);
  });
});
