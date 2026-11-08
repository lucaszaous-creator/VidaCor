import { screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Indicadores (Indicadores.dc.html)', () => {
  it('mostra os agregados do servidor e destaca laudo médio acima de 8 h', async () => {
    const { container } = renderizar('/indicadores');
    const resumo = await screen.findByRole('region', { name: 'Resumo' });
    expect(resumo).toHaveTextContent('Faltas9 (8%)');
    expect(resumo).toHaveTextContent('R$ 44.800,00');
    expect(resumo).toHaveTextContent('6,4 h');
    const medicos = screen.getByRole('grid', { name: 'Laudo por médico' });
    expect(within(medicos).getByText('11,5 h, acima de 8 h')).toBeInTheDocument();
    expect(screen.getByRole('table', { name: 'Agendamentos por dia da semana e hora' })).toHaveTextContent('08h');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('recepção não vê indicadores', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/indicadores');
    expect(await screen.findByText('Você não vê indicadores')).toBeInTheDocument();
  });
});
