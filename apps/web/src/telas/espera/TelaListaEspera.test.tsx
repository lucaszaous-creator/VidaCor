import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Lista de espera (Lista-Espera.dc.html)', () => {
  it('encaixa a vaga oferecida só com nome e nascimento conferidos', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/lista-espera');
    await usuario.click(await screen.findByRole('button', { name: /Maria Helena Duarte, nascimento 12\/03\/1968/ }));
    const painel = screen.getByRole('region', { name: 'Pedido de espera' });
    expect(within(painel).getByText(/Vaga oferecida: Sala 1/)).toBeInTheDocument();
    await usuario.click(within(painel).getByRole('button', { name: 'Encaixar Maria Helena Duarte' }));
    expect(within(painel).getByText('Escreva o nome completo.')).toBeInTheDocument();
    expect(pedidosDe('aceitar-oferta')).toHaveLength(0);
    await usuario.type(within(painel).getByRole('textbox', { name: 'Nome dito pelo paciente' }), 'Maria Helena Duarte');
    await digitarData(usuario, 'Nascimento dito pelo paciente', '13031968', painel);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(painel).getByRole('button', { name: 'Encaixar Maria Helena Duarte' }));
    await esperarAlertaDoServidor('Nome ou data de nascimento não conferem', 'identificacao-nao-confere');
  });

  it('urgente pede justificativa e médico antes de enviar', async () => {
    const usuario = userEvent.setup();
    renderizar('/lista-espera');
    const form = await screen.findByRole('form', { name: 'Incluir paciente na espera' });
    await usuario.type(within(form).getByRole('searchbox', { name: 'Paciente' }), 'Vera');
    await usuario.click(await screen.findByRole('button', { name: /^Vera Lúcia Antunes, nascimento 18\/09\/1957/ }));
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Exame' }), '40901114');
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Prioridade clínica' }), 'urgente');
    await usuario.click(within(form).getByRole('button', { name: 'Incluir na espera' }));
    expect(within(form).getByText('Urgente pede a justificativa.')).toBeInTheDocument();
    expect(pedidosDe('incluir-espera')).toHaveLength(0);
    await usuario.type(within(form).getByRole('textbox', { name: 'Justificativa' }), 'suspeita de valvopatia');
    await usuario.type(within(form).getByRole('textbox', { name: 'Médico solicitante' }), 'Dr. Paulo Medeiros');
    await usuario.click(within(form).getByRole('button', { name: 'Incluir na espera' }));
    expect(await screen.findByText('Vera Lúcia Antunes entrou na lista de espera.')).toBeInTheDocument();
    expect(pedidosDe('incluir-espera')[0]?.corpo).toMatchObject({ prioridade: 'urgente', medico_solicitante: 'Dr. Paulo Medeiros' });
  });

  it('bloqueio manda o horário de Brasília com motivo', async () => {
    const usuario = userEvent.setup();
    renderizar('/lista-espera');
    await usuario.click(await screen.findByRole('tab', { name: 'Bloquear horário' }));
    const form = screen.getByRole('form', { name: 'Bloquear horário' });
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Sala' }), within(form).getAllByRole('option')[1] as HTMLOptionElement);
    await digitarData(usuario, 'Dia', '20112026', form);
    await usuario.type(within(form).getByRole('textbox', { name: 'De (hh:mm)' }), '14:00');
    await usuario.type(within(form).getByRole('textbox', { name: 'Até (hh:mm)' }), '16:00');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo do bloqueio' }), 'manutenção preventiva');
    await usuario.click(within(form).getByRole('button', { name: 'Bloquear horário' }));
    expect(await screen.findByText(/Horário bloqueado: Sala 2\./)).toBeInTheDocument();
    expect(pedidosDe('bloquear')[0]?.corpo).toMatchObject({ tipo: 'manutencao', inicio: '2026-11-20T14:00:00-03:00', fim: '2026-11-20T16:00:00-03:00' });
  });
});
