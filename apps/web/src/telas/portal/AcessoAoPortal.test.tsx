import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { TOKEN_VALIDO } from '../../../testes/nucleo-portal';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Acesso ao portal: convite e login (SEC-AUT-6)', () => {
  it('login certo leva aos resultados e mostra quem entrou', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/portal/entrar');
    await usuario.type(await screen.findByRole('textbox', { name: 'Login' }), 'antonio.ribeiro');
    await usuario.type(screen.getByLabelText('Senha'), 'uma senha longa 2026');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Entrar' }));
    expect(await screen.findByRole('heading', { name: 'Seus resultados' })).toBeInTheDocument();
    expect(await screen.findByText('Olá, Antônio')).toBeInTheDocument();
  });

  it('login errado mostra a mensagem única do servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/portal/entrar');
    await usuario.type(await screen.findByRole('textbox', { name: 'Login' }), 'quem.sabe');
    await usuario.type(screen.getByLabelText('Senha'), 'qualquer coisa');
    await usuario.click(screen.getByRole('button', { name: 'Entrar' }));
    await esperarAlertaDoServidor('Login ou senha não conferem.', 'login-nao-confere');
  });

  it('convite: confere o nascimento, cria o acesso e entra', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(`/portal/convite/${TOKEN_VALIDO}`);
    expect(await screen.findByRole('heading', { name: 'Primeiro acesso' })).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Criar acesso e entrar' }));
    expect(screen.getByText('Digite a sua data de nascimento.')).toBeInTheDocument();
    expect(pedidosDe('portal-aceite')).toHaveLength(0);
    const dia = screen.getAllByRole('spinbutton');
    await usuario.click(dia[0] as HTMLElement);
    await usuario.keyboard('15061954');
    await usuario.type(screen.getByRole('textbox', { name: 'Login' }), 'antonio.ribeiro');
    await usuario.type(screen.getByLabelText('Senha'), 'uma senha longa 2026');
    await usuario.type(screen.getByLabelText('Repita a senha'), 'uma senha longa 2026');
    await usuario.click(screen.getByRole('button', { name: 'Criar acesso e entrar' }));
    expect(await screen.findByRole('heading', { name: 'Seus resultados' })).toBeInTheDocument();
    expect(pedidosDe('portal-aceite')[0]?.corpo).toEqual({ nascimento: '1954-06-15', login: 'antonio.ribeiro', senha: 'uma senha longa 2026' });
  });

  it('convite vencido ou usado diz o que fazer, sem pedir dados', async () => {
    renderizar('/portal/convite/token-que-nao-vale-mais-000000000000');
    expect(await screen.findByText(/Este convite não vale mais\./)).toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'Login' })).not.toBeInTheDocument();
  });

  it('recepção gera o convite e vê o link uma vez, com validade', async () => {
    estado.sessao = SESSOES.recepcao;
    const usuario = userEvent.setup();
    renderizar('/recepcao?data=2026-11-06');
    await usuario.click(await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento/ }));
    await usuario.click(await screen.findByRole('button', { name: 'Convidar para o portal' }));
    const aviso = await screen.findByText(/Convite do portal para Antônio Carlos Ribeiro\./);
    await waitFor(() => {
      expect(aviso.closest('[role="status"]')).toHaveTextContent(`/portal/convite/${TOKEN_VALIDO}`);
    });
    expect(aviso.closest('[role="status"]')).toHaveTextContent('uma vez só');
    expect(pedidosDe('convidar-portal')[0]?.chave).toMatch(/^[0-9a-f-]{36}$/);
  });
});
