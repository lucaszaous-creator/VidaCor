import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { comPropostaPendente } from '../../../testes/nucleo-convenios';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.faturista;
});

describe('Convênios e tabelas (Convenios-Tabelas.dc.html)', () => {
  it('o faturamento vê operadoras, a tabela vigente com o preço anterior e as outras abas', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/convenios');
    const lista = await screen.findByRole('region', { name: 'Operadoras' });
    expect(await within(lista).findByRole('button', { name: /SulAmérica.*Renovar/ })).toBeInTheDocument();
    const detalhe = screen.getByRole('region', { name: 'Detalhe do convênio' });
    expect(await within(detalhe).findByRole('heading', { name: 'Unimed' })).toBeInTheDocument();
    expect(within(detalhe).getByText('Vigente, até 31/12/2027')).toBeInTheDocument();
    expect(await within(detalhe).findByText('Ecocardiograma transtorácico')).toBeInTheDocument();
    expect(within(detalhe).getByText('R$ 248,60')).toBeInTheDocument();
    await usuario.click(within(detalhe).getByRole('tab', { name: 'Autorização' }));
    expect(within(detalhe).getAllByText('Pede senha').length).toBeGreaterThan(0);
    await usuario.click(within(detalhe).getByRole('tab', { name: 'Auditoria' }));
    expect(within(detalhe).getAllByText('Bloqueia').length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: 'Cadastrar operadora' })).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('o faturamento propõe reajuste em lote com a prévia do servidor e a chave de idempotência', async () => {
    const usuario = userEvent.setup();
    renderizar('/convenios');
    const detalhe = await screen.findByRole('region', { name: 'Detalhe do convênio' });
    await usuario.click(await within(detalhe).findByRole('button', { name: 'Reajuste em lote' }));
    const form = within(detalhe).getByRole('form', { name: 'Reajuste em lote' });
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para segunda pessoa aprovar' }));
    expect(within(form).getByText('Percentual como 5,20.')).toBeInTheDocument();
    await usuario.type(within(form).getByRole('textbox', { name: 'Percentual de reajuste' }), '5,20');
    expect(await within(form).findByText(/Prévia calculada pelo servidor sobre a versão 14: 142 procedimentos/)).toBeInTheDocument();
    await digitarData(usuario, 'Vigência a partir de', '01012027', form);
    await usuario.type(within(form).getByRole('textbox', { name: 'Nome da versão' }), 'Unimed 2027');
    await usuario.type(within(form).getByRole('textbox', { name: 'Documento' }), 'Carta de reajuste 2027');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'reajuste anual pelo IPCA');
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para segunda pessoa aprovar' }));
    expect(await screen.findByText('Proposta PTB-2026-000004 enviada para aprovação.')).toBeInTheDocument();
    const [pedido] = pedidosDe('propor-tabela');
    expect(pedido?.corpo).toEqual({ vigente_desde: '2027-01-01', descricao: 'Unimed 2027', documento: 'Carta de reajuste 2027', motivo: 'reajuste anual pelo IPCA', reajuste_pontos_base: 520 });
    expect(pedido?.chave).toMatch(/^[0-9a-f-]{36}$/);
  });

  it('com proposta pendente, o faturamento só vê que aguarda a gerência', async () => {
    comPropostaPendente();
    renderizar('/convenios');
    const proposta = await screen.findByRole('region', { name: 'Proposta aguardando aprovação' });
    expect(within(proposta).getByText('Aguardando a aprovação de uma pessoa da gerência.')).toBeInTheDocument();
    expect(within(proposta).queryByRole('button', { name: 'Aprovar tabela' })).not.toBeInTheDocument();
  });

  it('a gerência aprova a proposta pendente', async () => {
    comPropostaPendente();
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    const { container } = renderizar('/convenios');
    const proposta = await screen.findByRole('region', { name: 'Proposta aguardando aprovação' });
    expect(within(proposta).getByText(/Reajuste de 5,20% para Unimed, a partir de 01\/01\/2027/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Cadastrar operadora' })).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(proposta).getByRole('button', { name: 'Aprovar tabela' }));
    expect(await screen.findByText('Tabela v15 aprovada; começa em 01/01/2027.')).toBeInTheDocument();
    expect(pedidosDe('decidir-tabela')[0]?.corpo).toEqual({ aprovar: true, motivo: null });
  });

  it('cadastrar operadora já cadastrada mostra a mensagem do servidor, sem perder o que foi digitado', async () => {
    const usuario = userEvent.setup();
    renderizar('/convenios');
    await usuario.click(await screen.findByRole('button', { name: 'Cadastrar operadora' }));
    const form = screen.getByRole('form', { name: 'Cadastrar operadora' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Operadora' }), 'Unimed');
    await usuario.type(within(form).getByRole('textbox', { name: 'Registro na ANS' }), '345678');
    await usuario.type(within(form).getByRole('textbox', { name: 'Código do prestador na operadora' }), '0004567');
    await usuario.type(within(form).getByRole('textbox', { name: 'CNES' }), '1234567');
    await usuario.type(within(form).getByRole('textbox', { name: 'Nome do prestador na operadora' }), 'VidaCor Imagem');
    await usuario.selectOptions(await within(form).findByRole('combobox', { name: 'Regras da auditoria' }), 'AUD-UNIMED-2026.1');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'credenciamento novo');
    await usuario.click(within(form).getByRole('button', { name: 'Cadastrar' }));
    await esperarAlertaDoServidor('Esta operadora já tem convênio nesta unidade.', 'operadora-ja-cadastrada', form);
    expect(within(form).getByRole('textbox', { name: 'Operadora' })).toHaveValue('Unimed');
  });

  it('a recepção não consulta convênios: a tela diz e não mostra dado', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/convenios');
    expect(await screen.findByText('Você não consulta os convênios')).toBeInTheDocument();
    expect(screen.queryByText('Unimed')).not.toBeInTheDocument();
  });
});
