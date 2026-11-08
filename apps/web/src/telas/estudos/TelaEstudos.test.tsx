import { screen, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES, SONIA } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = '/estudos?data=2026-11-06';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medico;
});

async function conciliar(usuario: UserEvent, nascimento: string): Promise<void> {
  await usuario.click(await screen.findByRole('button', { name: /EST-2026-000081/ }));
  const form = screen.getByRole('form', { name: 'Conciliar o estudo EST-2026-000081' });
  await within(form).findByRole('option', { name: /Sônia Maria Castro$/ });
  await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Atendimento do dia' }), SONIA.atendimento_id ?? '');
  await usuario.type(within(form).getByRole('textbox', { name: 'Nome como está no estudo' }), 'Sônia Maria Castro');
  await digitarData(usuario, 'Nascimento como está no estudo', nascimento, form);
  await usuario.click(within(form).getByRole('button', { name: 'Conferir e conciliar' }));
  const dialogo = await screen.findByRole('alertdialog', { name: 'Vincular o estudo EST-2026-000081 a Sônia Maria Castro?' });
  expect(dialogo).toHaveTextContent('Sônia Maria Castro, nascimento 03/02/1962');
  await usuario.click(within(dialogo).getByRole('button', { name: 'Vincular estudo' }));
}

describe('Conciliação manual de estudos (docs/30 § 7)', () => {
  it('lista os pendentes e concilia com os dois identificadores e confirmação nominal', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    expect(await screen.findByText('Patient ID C-0414 sem accession')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await conciliar(usuario, '03021962');
    expect(await screen.findByText(/Estudo conciliado\./)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /EST-2026-000081/ })).not.toBeInTheDocument();
    const pedido = pedidosDe('conciliacao')[0];
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo).toEqual({ atendimento_id: SONIA.atendimento_id, nome: 'Sônia Maria Castro', nascimento: '1962-02-03' });
  });

  it('identificadores que não conferem (422) não conciliam e mostram a mensagem do servidor', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    await conciliar(usuario, '03021963');
    await esperarAlertaDoServidor('O nome ou o nascimento não confere com o atendimento escolhido.', 'identificacao-nao-confere');
    expect(screen.getByRole('button', { name: /EST-2026-000081/ })).toBeInTheDocument();
  });

  it('perfil sem permissão vê o estado de sem permissão', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar(URL);
    expect(await screen.findByText('Você não concilia estudos')).toBeInTheDocument();
  });
});
