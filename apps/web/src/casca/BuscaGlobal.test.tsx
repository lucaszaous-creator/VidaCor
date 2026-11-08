import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { reiniciarNucleo } from '../../testes/nucleo-simulado';
import { renderizar } from '../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Busca global (Busca-Global.dc.html)', () => {
  it('Ctrl+K abre, acha o paciente pelos dois identificadores e leva à tela dele', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/contingencia');
    await screen.findByRole('navigation', { name: 'Menu principal' });
    await usuario.keyboard('{Control>}k{/Control}');
    const caixa = await screen.findByRole('searchbox', { name: 'Buscar pacientes, códigos ou telas' });
    await usuario.type(caixa, 'Vera');
    const resultados = await screen.findByRole('list', { name: 'Resultados' });
    const item = await within(resultados).findByRole('button', { name: /Vera Lúcia Antunes nascimento 18\/09\/1957/ });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(item);
    expect(await screen.findByRole('heading', { name: 'Paciente' })).toBeInTheDocument();
  });

  it('com ">" lista só as telas do perfil', async () => {
    const usuario = userEvent.setup();
    renderizar('/contingencia');
    await usuario.click(await screen.findByRole('button', { name: 'Buscar em tudo (Ctrl K)' }));
    await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar pacientes, códigos ou telas' }), '>orç');
    const telas = await screen.findByRole('list', { name: 'Telas' });
    expect(within(telas).getByRole('button', { name: 'Orçamento particular' })).toBeInTheDocument();
    expect(within(telas).queryByRole('button', { name: 'Laudar' })).not.toBeInTheDocument();
  });
});
