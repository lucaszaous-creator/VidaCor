import { screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../testes/axe';
import { contingenciaAberta, SESSOES } from '../../testes/dados/fase1';
import { fase1 } from '../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../testes/nucleo-simulado';
import { renderizar } from '../../testes/renderizar';
import { itemAtivo, secoesDoMenu, telaInicial } from './menu';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Menu lateral por perfil (conforto; o servidor decide)', () => {
  it.each([
    ['recepção', SESSOES.recepcao, ['Início', 'Agenda e check-in', 'Novo agendamento', 'Pacientes', 'Lista de espera', 'Orçamento particular', 'Exame externo', 'Novo paciente', 'Recepção e pulseiras', 'TV de chamada', 'Autorizações', 'Exames e procedimentos', 'Qualidade', 'Ouvidoria', 'Contingência']],
    ['técnico', SESSOES.tecnico, ['Agenda e check-in', 'Pacientes', 'Sala de exame', 'Qualidade']],
    ['médico', SESSOES.medico, ['Agenda e check-in', 'Pacientes', 'Sala de exame', 'Laudar', 'Conciliar estudos', 'Qualidade']],
    ['caixa', SESSOES.caixa, ['Caixa (particular)', 'Qualidade']],
    ['gerência', SESSOES.gerente, ['Início', 'Agenda e check-in', 'Novo agendamento', 'Pacientes', 'Lista de espera', 'Orçamento particular', 'Exame externo', 'Novo paciente', 'Recepção e pulseiras', 'TV de chamada', 'Autorizações', 'Fluxo de laudos', 'Financeiro', 'Indicadores', 'Estoque', 'Pendências do caixa', 'Faturamento TISS', 'Auditoria do faturamento', 'Configurações', 'Aparelhos', 'Usuários e perfis', 'Corpo clínico', 'Exames e procedimentos', 'Qualidade', 'Ouvidoria', 'Lastro', 'Contingência']],
    ['TI', SESSOES.ti, ['Configurações', 'Aparelhos', 'Usuários e perfis', 'Qualidade']],
    ['faturista', SESSOES.faturista, ['Autorizações', 'Faturamento TISS', 'Auditoria do faturamento', 'Qualidade']],
  ])('%s vê só os módulos do perfil', (_nome, sessao, esperados) => {
    expect(secoesDoMenu(sessao.usuario.perfis).flatMap((s) => s.itens.map((i) => i.rotulo))).toEqual(esperados);
  });

  it('na tela, a recepção vê só os grupos dos seus módulos', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/contingencia');
    const menu = await screen.findByRole('navigation', { name: 'Menu principal' });
    const grupos = within(menu)
      .getAllByRole('button')
      .filter((b) => b.hasAttribute('aria-expanded') && b.getAttribute('aria-label') === null)
      .map((b) => b.textContent);
    expect(grupos).toEqual(['Clínica', 'Qualidade']);
    expect(within(menu).getByRole('link', { name: 'Contingência' })).toHaveAttribute('aria-current', 'page');
    expect(within(menu).queryByRole('link', { name: 'Laudar' })).not.toBeInTheDocument();
  });

  it('perfil desconhecido não ganha item nenhum', () => {
    expect(secoesDoMenu(['visitante'])).toEqual([]);
    expect(telaInicial(['visitante'])).toBe('/agenda');
    expect(telaInicial(['caixa'])).toBe('/caixa');
    // A conta do dispositivo TV só tem o painel, e entra direto nele.
    expect(secoesDoMenu(['tv']).flatMap((s) => s.itens.map((i) => i.rotulo))).toEqual(['TV de chamada']);
    expect(telaInicial(['tv'])).toBe('/telao');
  });

  it('o item ativo segue a rota, inclusive a triagem dentro da sala', () => {
    const secoes = secoesDoMenu(['tecnico', 'medico', 'caixa', 'gerente']);
    expect(itemAtivo('/laudos/abc', secoes)).toBe('laudos');
    expect(itemAtivo('/caixa/pendencias', secoes)).toBe('pendencias');
    expect(itemAtivo('/caixa', secoes)).toBe('caixa');
    expect(itemAtivo('/triagem/abc', secoes)).toBe('sala');
  });

  it('cada módulo tem a cor do docs/05 e o item ativo é marcado', async () => {
    estado.sessao = SESSOES.medico;
    const { container } = renderizar('/laudos');
    const laudar = await screen.findByRole('link', { name: 'Laudar' });
    expect(laudar).toHaveAttribute('aria-current', 'page');
    expect(laudar.querySelector('.ni')).toHaveClass('k-violet');
    expect(screen.getByRole('link', { name: 'Sala de exame' }).querySelector('.ni')).toHaveClass('k-teal');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('faixa de contingência aberta aparece no topo de qualquer tela (R7)', async () => {
    fase1.contingencia = contingenciaAberta();
    estado.sessao = SESSOES.tecnico;
    renderizar('/agenda?data=2026-11-06');
    const faixa = await screen.findByText(/Contingência CTG-2026-0007 aberta/);
    expect(faixa.closest('[role="status"]')).toHaveTextContent('atinge toda a unidade: servidor local fora do ar. Declarada às 10:42.');
    expect(within(faixa.closest('[role="status"]') as HTMLElement).getByRole('link', { name: 'Abrir contingência' })).toHaveAttribute('href', '/contingencia');
  });
});
