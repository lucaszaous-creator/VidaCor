import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { lastroSimulado } from '../../../testes/nucleo-estoque';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

async function abrir(codigo: string) {
  const usuario = userEvent.setup();
  const r = renderizar('/lastro');
  await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar por qualquer código' }), codigo);
  await usuario.click(screen.getByRole('button', { name: 'Abrir cadeia' }));
  return r;
}

describe('Explorador de lastro (Lastro.dc.html)', () => {
  it('abre a cadeia pelo código, com paciente, registros e eventos conferidos', async () => {
    const { container } = await abrir('ATD-2026-000031');
    expect(await screen.findByRole('heading', { name: 'Sônia Maria Castro' })).toBeInTheDocument();
    expect(screen.getByText(/Cadeia íntegra\./)).toBeInTheDocument();
    const eventos = screen.getByRole('region', { name: 'Eventos' });
    expect(within(eventos).getAllByText('Confere')).toHaveLength(2);
    expect(within(screen.getByRole('region', { name: 'Cadeia do atendimento' })).getByText('Etiqueta')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('evento que não confere aparece em destaque e a tela diz o que fazer', async () => {
    lastroSimulado.integra = false;
    await abrir('ATD-2026-000031');
    expect(await screen.findByText(/Há evento que não confere com o hash\./)).toBeInTheDocument();
    expect(screen.getByText('Não confere')).toBeInTheDocument();
  });

  it('código desconhecido mostra a mensagem do servidor; recepção não abre', async () => {
    await abrir('XYZ-1');
    expect(await screen.findByText(/Nenhum atendimento desta unidade tem este código\./)).toBeInTheDocument();
  });
});
