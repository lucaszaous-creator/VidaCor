import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Usuários e perfis (Usuarios-Perfis.dc.html)', () => {
  it('a gerência confere o médico novo que aguarda a segunda pessoa', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/usuarios');
    await usuario.click(await screen.findByRole('button', { name: /Dra\. Renata Alves/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do usuário' });
    expect(within(detalhe).getByText(/Aguarda a conferência de outra pessoa: criar\./)).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Conferir e aplicar' }));
    expect(within(detalhe).getByText('Diga o motivo (mínimo 5 letras).')).toBeInTheDocument();
    await usuario.type(within(detalhe).getByRole('textbox', { name: 'Motivo da mudança' }), 'conferido com o contrato');
    await usuario.click(within(detalhe).getByRole('button', { name: 'Conferir e aplicar' }));
    expect(await screen.findByText('Mudança registrada no lastro.')).toBeInTheDocument();
    expect(pedidosDe('decidir-perfil')[0]?.corpo).toEqual({ aprovar: true, motivo: 'conferido com o contrato' });
  });

  it('separação de funções vem do servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/usuarios');
    await usuario.click(await screen.findByRole('button', { name: /Carla Mendes/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do usuário' });
    await usuario.type(within(detalhe).getByRole('textbox', { name: 'Motivo da mudança' }), 'cobrir férias');
    await usuario.selectOptions(within(detalhe).getByRole('combobox', { name: 'Adicionar perfil' }), 'financeiro');
    await usuario.click(within(detalhe).getByRole('button', { name: 'Adicionar' }));
    await esperarAlertaDoServidor('Quem recebe no caixa não concilia no financeiro.', 'separacao-de-funcoes');
  });

  it('a TI cria usuário e é avisada de que perfil clínico espera conferência', async () => {
    estado.sessao = SESSOES.ti;
    const usuario = userEvent.setup();
    renderizar('/usuarios');
    await usuario.click(await screen.findByRole('button', { name: 'Novo usuário' }));
    await usuario.type(screen.getByRole('textbox', { name: 'Login (o mesmo do acesso da clínica)' }), 'luis.tavares');
    await usuario.type(screen.getByRole('textbox', { name: 'Nome completo' }), 'Dr. Luís Tavares');
    await usuario.click(screen.getByRole('checkbox', { name: 'Médico' }));
    expect(screen.getByText(/O usuário nasce inativo/)).toBeInTheDocument();
    await usuario.type(screen.getByRole('textbox', { name: 'Motivo' }), 'contratação');
    await usuario.click(screen.getByRole('button', { name: 'Criar usuário' }));
    expect(await screen.findByRole('heading', { name: 'Dr. Luís Tavares' })).toBeInTheDocument();
    expect(pedidosDe('criar-usuario')[0]?.corpo).toEqual({ login: 'luis.tavares', nome: 'Dr. Luís Tavares', perfis: ['medico'], motivo: 'contratação' });
  });
});
