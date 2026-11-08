import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Paciente 360 (Paciente.dc.html)', () => {
  it('busca o paciente e mostra os dois identificadores, a alergia, os exames e o CPF mascarado', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/pacientes');
    await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar paciente' }), 'Vera');
    await usuario.click(await screen.findByRole('button', { name: /^Vera Lúcia Antunes, nascimento 18\/09\/1957/ }));
    expect(await screen.findByText(/Alergia: dipirona/)).toBeInTheDocument();
    const exames = screen.getByRole('region', { name: 'Exames do paciente' });
    expect(within(exames).getByText('MAPA 24 h')).toBeInTheDocument();
    await usuario.click(within(exames).getByRole('tab', { name: 'Histórico (1)' }));
    expect(within(exames).getByText('Ecocardiograma transtorácico')).toBeInTheDocument();
    expect(within(exames).getByText(/1 exame\(s\) externo\(s\) importado\(s\)/)).toBeInTheDocument();
    const cadastro = screen.getByRole('region', { name: 'Cadastro e contato' });
    expect(within(cadastro).getByText('***.***.***-25')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });
});
