import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { problema, servidor } from '../../testes/nucleo-simulado';
import { renderizar } from '../../testes/renderizar';

describe('Tela de entrada', () => {
  it('mostra o servidor conectado e leva ao login da clínica no servidor', async () => {
    const usuario = userEvent.setup();
    const { irPara, container } = renderizar('/entrar');
    expect(await screen.findByText('Servidor local conectado')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: 'Entrar' }));
    expect(irPara).toHaveBeenCalledWith(`${window.location.origin}/v1/sessao/entrar`);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('diz quando o servidor não responde, sem esconder o botão de entrar', async () => {
    servidor.use(http.get('*/v1/saude', () => HttpResponse.error()));
    renderizar('/entrar');
    expect(await screen.findByText('Servidor local sem resposta')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent('O servidor local não respondeu.');
    expect(screen.getByRole('button', { name: 'Entrar' })).toBeEnabled();
  });

  it('sem sessão, a agenda manda para a entrada', async () => {
    servidor.use(http.get('*/v1/sessao', () => problema(401, 'sessao-ausente', 'Entre para continuar.')));
    renderizar('/agenda?data=2026-11-06');
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Entrar' })).toBeInTheDocument();
    });
  });
});
