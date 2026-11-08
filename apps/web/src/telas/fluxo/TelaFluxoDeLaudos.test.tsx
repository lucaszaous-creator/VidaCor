import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Fluxo de laudos (Fluxo-Laudos.dc.html)', () => {
  it('mostra o quadro por etapa com o prazo e filtra por médico', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/fluxo-de-laudos');
    const aLaudar = await screen.findByRole('list', { name: 'A laudar' });
    expect(within(aLaudar).getAllByRole('listitem')).toHaveLength(2);
    expect(within(aLaudar).getByText('Fora do prazo')).toBeInTheDocument();
    expect(screen.getByText(/87% no prazo hoje, 0 vencem em 2 h, 1 fora do prazo/)).toBeInTheDocument();
    await usuario.selectOptions(screen.getByRole('combobox', { name: 'Médico' }), 'Dra. Cecília Souto');
    expect(within(screen.getByRole('list', { name: 'A laudar' })).getAllByRole('listitem')).toHaveLength(1);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('tab', { name: 'Retificações (1)' }));
    expect(screen.getByText(/Lateralidade corrigida\. A versão anterior segue guardada\./)).toBeInTheDocument();
  });
});
