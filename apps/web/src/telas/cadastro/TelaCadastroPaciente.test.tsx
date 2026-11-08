import { screen, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.recepcao;
});

async function preencher(usuario: UserEvent, nome: string, data: string): Promise<void> {
  await usuario.type(await screen.findByRole('textbox', { name: /Nome completo/ }), nome);
  const segmentos = within(screen.getByRole('group', { name: /Data de nascimento/ })).getAllByRole('spinbutton');
  await usuario.click(segmentos[0] as HTMLElement);
  await usuario.keyboard(data);
  await usuario.selectOptions(screen.getByRole('combobox', { name: /Sexo/ }), 'F');
  await usuario.type(screen.getByRole('textbox', { name: /Celular/ }), '11987654321');
  await usuario.click(screen.getByRole('checkbox', { name: 'Paciente nega alergias' }));
}

describe('Cadastro de paciente (Cadastro-Paciente.dc.html)', () => {
  it('cadastra sem duplicado e oferece agendar', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/pacientes/novo');
    await preencher(usuario, 'Beatriz Nogueira Alves', '21081979');
    expect(await screen.findByText('Nenhum cadastro parecido encontrado.')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Salvar cadastro' }));
    expect(await screen.findByText(/Beatriz Nogueira Alves cadastrado\./)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Agendar exame' })).toHaveAttribute('href', '/agendamentos/novo');
    expect(pedidosDe('cadastrar-paciente')[0]?.corpo).toMatchObject({
      nome: 'Beatriz Nogueira Alves',
      nascimento: '1979-08-21',
      sexo: 'F',
      alergias: [],
      nega_alergias: true,
      duplicado_conferido: false,
    });
  });

  it('possível duplicado trava o salvar até a recepção conferir', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacientes/novo');
    await preencher(usuario, 'Maria Helena Duarte', '21031968');
    const aviso = await screen.findByText(/Possível duplicado\./);
    expect(aviso.closest('[role="status"], [role="alert"]')).toHaveTextContent('Maria Helena Duarte, nascimento 12/03/1968, PAC-2026-000002');
    expect(screen.getByRole('button', { name: 'Salvar cadastro' })).toBeDisabled();
    await usuario.click(screen.getByRole('button', { name: 'Não é a mesma pessoa' }));
    await usuario.click(screen.getByRole('button', { name: 'Salvar cadastro' }));
    expect(await screen.findByText(/Maria Helena Duarte cadastrado\./)).toBeInTheDocument();
    expect(pedidosDe('cadastrar-paciente')[0]?.corpo).toMatchObject({ duplicado_conferido: true });
  });

  it('campos obrigatórios dizem o que falta antes de enviar', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacientes/novo');
    await usuario.click(await screen.findByRole('button', { name: 'Salvar cadastro' }));
    expect(screen.getByText('Digite o nome completo, com sobrenome.')).toBeInTheDocument();
    expect(screen.getByText('Informe as alergias ou marque que o paciente nega alergias.')).toBeInTheDocument();
    expect(pedidosDe('cadastrar-paciente')).toHaveLength(0);
  });
});
