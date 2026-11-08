import { screen, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SALA_1, SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

async function preencherPedido(usuario: UserEvent, form: HTMLElement, operadora: string, uf: string): Promise<void> {
  await usuario.type(within(form).getByRole('textbox', { name: 'Operadora do pedido' }), operadora);
  await usuario.type(within(form).getByRole('textbox', { name: 'Carteirinha do pedido' }), '0 123 4567');
  await usuario.type(within(form).getByRole('textbox', { name: 'Médico solicitante' }), 'Dr. Júlio Prado');
  await usuario.type(within(form).getByRole('textbox', { name: 'Número no conselho' }), '52123456');
  await usuario.type(within(form).getByRole('textbox', { name: 'UF do conselho (código IBGE)' }), uf);
  await usuario.type(within(form).getByRole('textbox', { name: 'CBO do solicitante' }), '225125');
}

describe('Autorização no agendamento (Agendamento-Guia.dc.html)', () => {
  it('a reserva sem guia oferece pedir a autorização; o servidor escolhe o canal e a guia vem da resposta', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/agendamentos/novo');
    await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar paciente por nome' }), 'Antônio');
    await usuario.click(await screen.findByRole('button', { name: 'Antônio Carlos Ribeiro, nascimento 15/06/1954' }));
    await usuario.selectOptions(screen.getByRole('combobox', { name: 'Sala' }), SALA_1.id);
    await digitarData(usuario, 'Dia do exame', '07112026');
    await usuario.click(await screen.findByRole('button', { name: '09:00' }));
    await usuario.type(screen.getByRole('textbox', { name: 'Código TUSS 1' }), '40901114');
    await usuario.click(screen.getByRole('button', { name: 'Reservar sem guia' }));
    const form = await screen.findByRole('form', { name: 'Pedir autorização' });
    await usuario.click(within(form).getByRole('button', { name: 'Pedir autorização' }));
    expect(within(form).getAllByText('Preencha este campo.').length).toBeGreaterThan(0);
    expect(pedidosDe('autorizacao')).toHaveLength(0);

    await preencherPedido(usuario, form, 'Unimed', 'RJ');
    await usuario.click(within(form).getByRole('button', { name: 'Pedir autorização' }));
    await esperarAlertaDoServidor('Os dados do médico solicitante não estão no padrão TISS', 'solicitante-invalido', form);
    await usuario.clear(within(form).getByRole('textbox', { name: 'UF do conselho (código IBGE)' }));
    await usuario.type(within(form).getByRole('textbox', { name: 'UF do conselho (código IBGE)' }), '33');
    await usuario.click(within(form).getByRole('button', { name: 'Pedir autorização' }));
    expect(await screen.findByText(/Pedido de autorização AUT-2026-000005 criado\./)).toBeInTheDocument();
    expect(screen.getByText(/Enviado pelo webservice TISS\. A guia é preenchida sozinha/)).toBeInTheDocument();
    const pedidos = pedidosDe('autorizacao');
    expect(pedidos[1]?.chave).toMatch(UUID);
    expect(pedidos[1]?.chave).not.toBe(pedidos[0]?.chave);
    expect(pedidos[1]?.corpo).toMatchObject({
      operadora: 'Unimed',
      carteirinha: '0 123 4567',
      solicitante: { nome: 'Dr. Júlio Prado', conselho: '06', numero: '52123456', uf: '33', cbos: '225125' },
      indicacao_clinica: null,
    });
    expect(pedidos[1]?.corpo).not.toHaveProperty('procedimentos');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });
});

describe('Autorizações (Autorizacao.dc.html)', () => {
  it('negada aparece primeiro com o motivo da operadora; autorizada mostra a guia preenchida sem digitação', async () => {
    estado.sessao = SESSOES.recepcao;
    const usuario = userEvent.setup();
    const { container } = renderizar('/autorizacoes');
    const lista = await screen.findByRole('region', { name: 'Pedidos de autorização' });
    const linhas = await within(lista).findAllByRole('button', { name: /nascimento/ });
    expect(linhas[0]).toHaveAccessibleName(/^Maria Helena Duarte, nascimento 12\/03\/1968/);
    await usuario.click(linhas[0] as HTMLElement);
    const detalhe = screen.getByRole('region', { name: 'Detalhe da autorização' });
    expect(await within(detalhe).findByText('A operadora negou o pedido.')).toBeInTheDocument();
    expect(within(detalhe).getByText(/1423\. Procedimento fora da cobertura do plano/)).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);

    await usuario.click(within(lista).getByRole('button', { name: /^Antônio Carlos Ribeiro/ }));
    expect(await within(detalhe).findByText('Guia preenchida sem digitação')).toBeInTheDocument();
    expect(within(detalhe).getByText(/Guia OP2026000001, senha DEV-2026000001, válida até 08\/12\/2026; reserva confirmada/)).toBeInTheDocument();
  });

  it('em análise: consulta a operadora agora e mostra a próxima consulta automática', async () => {
    estado.sessao = SESSOES.faturista;
    const usuario = userEvent.setup();
    renderizar('/autorizacoes');
    await usuario.click(await screen.findByRole('button', { name: /^Vera Lúcia Antunes/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe da autorização' });
    expect(await within(detalhe).findByText('A operadora ainda não deu a resposta final.')).toBeInTheDocument();
    await usuario.click(within(detalhe).getByRole('button', { name: 'Consultar operadora agora' }));
    expect(await within(detalhe).findByText(/\(tentativa 0\)/)).toBeInTheDocument();
    expect(pedidosDe('consulta')).toHaveLength(1);
  });

  it('operadora sem webservice: registra o protocolo do portal e depois a resposta copiada', async () => {
    estado.sessao = SESSOES.recepcao;
    const usuario = userEvent.setup();
    const { container } = renderizar('/autorizacoes');
    await usuario.click(await screen.findByRole('button', { name: /^Sônia Maria Castro/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe da autorização' });
    const protocolo = await within(detalhe).findByRole('form', { name: 'Protocolo do portal' });
    await usuario.type(within(protocolo).getByRole('textbox', { name: 'Protocolo do portal' }), 'PB-2026-8812');
    await usuario.click(within(protocolo).getByRole('button', { name: 'Registrar protocolo' }));
    const resposta = await within(detalhe).findByRole('form', { name: 'Resposta do portal' });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(resposta).getByRole('radio', { name: 'Autorizada' }));
    await usuario.click(within(resposta).getByRole('button', { name: 'Registrar resposta' }));
    await esperarAlertaDoServidor('Autorizada pede a senha', 'resposta-invalida', resposta);
    await usuario.type(within(resposta).getByRole('textbox', { name: 'Senha' }), 'PB-10377');
    await digitarData(usuario, 'Válida até', '08122026', resposta);
    await usuario.click(within(resposta).getByRole('button', { name: 'Registrar resposta' }));
    expect(await within(detalhe).findByText('Guia preenchida sem digitação')).toBeInTheDocument();
    const r = pedidosDe('resposta-portal');
    expect(r[1]?.chave).toMatch(UUID);
    expect(r[1]?.corpo).toMatchObject({ situacao: 'autorizada', senha: 'PB-10377', validade: '2026-12-08', procedimentos: [{ tuss: '40901114', autorizado: 1 }, { tuss: '40901300', autorizado: 1 }] });
  });

  it('caixa e médico veem a recusa do servidor', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/autorizacoes');
    expect(await screen.findByText('Você não vê as autorizações')).toBeInTheDocument();
  });
});
