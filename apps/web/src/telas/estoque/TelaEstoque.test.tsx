import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { ATENDIMENTO_VERA, SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Estoque com lote (Estoque.dc.html)', () => {
  it('lotes na ordem de saída e entrada de lote pelo estoque, sem rastreio de paciente', async () => {
    estado.sessao = SESSOES.estoque;
    const usuario = userEvent.setup();
    const { container } = renderizar('/estoque');
    await usuario.click(await screen.findByRole('button', { name: /Contraste iodado não iônico 350/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do item' });
    const lotes = within(detalhe).getAllByText(/^L\d{4}-\d{2}$/).map((e) => e.textContent);
    expect(lotes).toEqual(['L2408-77', 'L2501-12']);
    expect(within(detalhe).queryByRole('button', { name: 'Rastrear lote' })).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.type(within(detalhe).getByRole('textbox', { name: 'Lote' }), 'g1120');
    await digitarData(usuario, 'Validade', '30112027');
    await usuario.type(within(detalhe).getByRole('textbox', { name: 'Quantidade' }), '20');
    await usuario.click(within(detalhe).getByRole('button', { name: 'Dar entrada' }));
    expect(await screen.findByText(/Lote G1120 com 20 em estoque\./)).toBeInTheDocument();
    expect(pedidosDe('entrada-estoque')[0]?.corpo).toMatchObject({ numero_lote: 'g1120', validade: '2027-11-30', quantidade: 20 });
  });

  it('gerência rastreia o lote e vê quem recebeu, com nascimento', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    renderizar('/estoque');
    await usuario.click(await screen.findByRole('button', { name: /Contraste iodado não iônico 350/ }));
    await usuario.click(screen.getAllByRole('button', { name: 'Rastrear lote' })[0] as HTMLElement);
    const r = await screen.findByText(/Lote L2408-77: 1 paciente\(s\)\./);
    expect(r.closest('[role="status"]')).toHaveTextContent('Sônia Maria Castro, nascimento 03/02/1962, ATD-2026-000031');
  });
});

describe('Contraste aplicado na triagem', () => {
  it('a enfermagem registra o contraste depois da triagem liberada; o lote vem do servidor', async () => {
    estado.sessao = SESSOES.enfermagem;
    const usuario = userEvent.setup();
    renderizar(`/triagem/${ATENDIMENTO_VERA.id}`);
    for (const [pergunta, resposta] of [
      ['Já teve reação a contraste iodado?', 'Não'],
      ['Existe possibilidade de gravidez?', 'Não'],
      ['Está amamentando?', 'Não'],
      ['Usa metformina?', 'Não'],
    ] as const) {
      await usuario.click(within(await screen.findByRole('radiogroup', { name: pergunta })).getByRole('radio', { name: resposta }));
    }
    await usuario.type(screen.getByRole('textbox', { name: 'Creatinina (mg/dL)' }), '0,7');
    await digitarData(usuario, 'Data do exame de creatinina', '28102026');
    await usuario.click(screen.getByRole('button', { name: 'Salvar triagem' }));
    const form = await screen.findByRole('form', { name: 'Contraste aplicado' });
    await screen.findByRole('option', { name: /Contraste iodado não iônico 350/ });
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Contraste' }), '00000028-0000-7000-8000-000000000001');
    await usuario.click(within(form).getByRole('button', { name: 'Registrar contraste' }));
    const feito = await screen.findByText(/Contraste registrado para Vera Lúcia Antunes\./);
    expect(feito.closest('[role="status"]')).toHaveTextContent('Lote L2408-77, validade 31/03/2027');
    expect(pedidosDe('contraste')[0]?.corpo).toEqual({ item_id: '00000028-0000-7000-8000-000000000001', quantidade: 1 });
  });
});
