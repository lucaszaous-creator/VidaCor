import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pacientesSimulados } from '../../../testes/nucleo-pacientes';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';
import { idade } from './ListaDePacientes';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Lista de pacientes (Pacientes-Lista.dc.html)', () => {
  it('mostra o total, os recortes com contagem e a situação que veio do servidor', async () => {
    const { container } = renderizar('/pacientes');
    expect(await screen.findByText('5 cadastrados')).toBeInTheDocument();
    const lista = screen.getByRole('region', { name: 'Lista de pacientes' });
    expect(within(lista).getByRole('button', { name: 'Vera Lúcia Antunes, nascimento 18/09/1957' })).toBeInTheDocument();
    expect(within(lista).getByText('Na clínica')).toBeInTheDocument();
    expect(within(lista).getByText('Reserva sem guia')).toBeInTheDocument();
    const recortes = screen.getByRole('navigation', { name: 'Recortes' });
    expect(within(recortes).getByRole('button', { name: /Cadastro incompleto 2/ })).toBeInTheDocument();
    expect(within(recortes).getByRole('button', { name: /Possíveis duplicados 1/ })).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('busca sem acento, mostra o resumo com dois identificadores e abre a ficha', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacientes');
    await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar paciente' }), 'vera lucia');
    const lista = screen.getByRole('region', { name: 'Lista de pacientes' });
    await usuario.click(await within(lista).findByRole('button', { name: 'Vera Lúcia Antunes, nascimento 18/09/1957' }));
    const resumo = screen.getByRole('complementary', { name: 'Resumo do paciente' });
    expect(within(resumo).getByRole('heading', { name: 'Vera Lúcia Antunes' })).toBeInTheDocument();
    expect(within(resumo).getByText('Alergia a dipirona')).toBeInTheDocument();
    expect(within(resumo).getByText('PAC-2026-000004')).toBeInTheDocument();
    expect(within(resumo).getByText('***.***.***-25')).toBeInTheDocument();
    await usuario.click(within(resumo).getByRole('link', { name: 'Abrir ficha' }));
    expect(await screen.findByText(/Alergia: dipirona/)).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Exames do paciente' })).toBeInTheDocument();
  });

  it('cadastro incompleto diz o que falta', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacientes');
    await usuario.click(await screen.findByRole('button', { name: /Cadastro incompleto 2/ }));
    const lista = screen.getByRole('region', { name: 'Lista de pacientes' });
    await usuario.click(await within(lista).findByRole('button', { name: 'Sônia Maria Castro, nascimento 03/02/1962' }));
    expect(screen.getByText('Falta no cadastro: CPF, celular, nome da mãe, alergias.')).toBeInTheDocument();
  });

  it('menu de contexto pelo teclado traz as ações do servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacientes');
    const lista = await screen.findByRole('region', { name: 'Lista de pacientes' });
    await usuario.click(await within(lista).findByRole('button', { name: 'Mais opções de Vera Lúcia Antunes' }));
    const menu = await screen.findByRole('menu');
    expect(await within(menu).findByRole('menuitem', { name: 'Abrir ficha' })).toBeInTheDocument();
    expect(within(menu).getByRole('menuitem', { name: 'Agendar exame' })).toBeInTheDocument();
  });

  it('decide sobre o possível duplicado e mostra o que foi feito', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacientes');
    await usuario.click(await screen.findByRole('button', { name: /Possíveis duplicados 1/ }));
    const par = await screen.findByRole('article', { name: 'Possível duplicado: Maria Helena Duarte' });
    expect(within(par).getByText('Mesmo nome e mesmo celular, nascimento diferente.')).toBeInTheDocument();
    expect(within(par).getByText('12/03/1968, prontuário PAC-2026-000002')).toBeInTheDocument();
    expect(within(par).getByText('21/03/1968, prontuário PAC-2026-000790')).toBeInTheDocument();
    await usuario.click(within(par).getByRole('button', { name: 'Pedir junção' }));
    expect(await within(par).findByText('Junção pedida.')).toBeInTheDocument();
    expect(within(par).getByText(/Aguarda a conferência de outra pessoa; até lá os cadastros seguem separados/)).toBeInTheDocument();
    expect(pacientesSimulados.decisoes).toEqual([expect.objectContaining({ decisao: 'juncao_pedida', motivo: null })]);
  });

  it('técnico vê a lista sem novo paciente nem a aba de duplicados', async () => {
    estado.sessao = SESSOES.tecnico;
    renderizar('/pacientes');
    expect(await screen.findByText('5 cadastrados')).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'Novo paciente' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Possíveis duplicados/ })).not.toBeInTheDocument();
  });

  it('caixa recebe a recusa do servidor, sem lista', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/pacientes');
    expect(await screen.findByText(/Seu perfil não pode ver a lista de pacientes/)).toBeInTheDocument();
  });

  it('idade em anos completos', () => {
    expect(idade('1968-03-12', new Date(2026, 10, 9))).toBe('58 anos');
    expect(idade('1968-11-10', new Date(2026, 10, 9))).toBe('57 anos');
  });
});
