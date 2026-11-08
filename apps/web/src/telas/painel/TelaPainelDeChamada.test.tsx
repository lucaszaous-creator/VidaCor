import { screen, within } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { chamadasSimuladas } from '../../../testes/nucleo-estoque';
import { estado, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.tv;
});

const chamada = (senha: string, destino: string, hora: string) => ({ senha, nome_abreviado: 'Antônio C. R.', destino, rechamada: false, chamada_em: `2026-11-06T${hora}:00Z` });

describe('TV de chamada (Telao.dc.html)', () => {
  it('mostra a chamada atual em destaque e as anteriores, sem menu nem dado clínico', async () => {
    chamadasSimuladas.push(chamada('A030', 'Guichê 1', '11:25'), chamada('A031', 'Sala 3', '11:33'), chamada('A012', 'Sala 1', '11:40'));
    const { container } = renderizar('/telao');
    const atual = await screen.findByRole('region', { name: 'Chamada atual' });
    expect(await within(atual).findByText('A012')).toBeInTheDocument();
    expect(within(atual).getByText('Sala 1')).toBeInTheDocument();
    expect(within(atual).getByText('Antônio C. R.')).toBeInTheDocument();
    const ultimas = within(screen.getByRole('complementary', { name: 'Últimas chamadas' })).getAllByRole('listitem');
    expect(ultimas.map((l) => l.querySelector('b')?.textContent)).toEqual(['A031', 'A030']);
    expect(screen.queryByRole('navigation', { name: 'Menu principal' })).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('sem conexão, diz que não está atualizando e que as chamadas seguem em voz alta', async () => {
    servidor.use(http.get('*/v1/painel-de-chamada', () => HttpResponse.error()));
    renderizar('/telao');
    expect(await screen.findByRole('status')).toHaveTextContent('As chamadas seguem em voz alta.');
  });

  it('sem chamada no dia, aguarda a primeira', async () => {
    renderizar('/telao');
    expect(await screen.findByText('Aguardando a primeira chamada')).toBeInTheDocument();
  });
});
