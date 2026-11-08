import { screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Início (Main.dc.html)', () => {
  it('recepção vê os números do dia e os próximos pacientes com os dois identificadores, sem o caixa', async () => {
    const { container } = renderizar('/inicio');
    expect(await screen.findByText('Exames agendados hoje')).toBeInTheDocument();
    const proximos = screen.getByRole('region', { name: 'Próximos pacientes' });
    expect(within(proximos).getByText('Vera Lúcia Antunes, nascimento 18/09/1957')).toBeInTheDocument();
    expect(within(proximos).getByText('Sem guia')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Ocupação da Sala 1' })).toHaveAttribute('aria-valuenow', '75');
    expect(screen.queryByRole('region', { name: 'Caixa do dia' })).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('gerência vê também o caixa do dia', async () => {
    estado.sessao = SESSOES.gerente;
    renderizar('/inicio');
    const caixa = await screen.findByRole('region', { name: 'Caixa do dia' });
    expect(within(caixa).getByText('R$ 3.784,00')).toBeInTheDocument();
  });
});
