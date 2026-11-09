import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { SHA_FICTICIO } from '../../../testes/nucleo-documentos';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Relatórios e exportação (Relatorios.dc.html)', () => {
  it('mostra o catálogo do perfil e a prévia; dado de saúde aparece só como código', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/relatorios');
    const catalogo = await screen.findByRole('navigation', { name: 'Catálogo de relatórios' });
    expect(within(catalogo).getAllByRole('button')).toHaveLength(4);
    expect(await screen.findByRole('table', { name: 'Prévia de Produção por médico' })).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(catalogo).getByRole('button', { name: 'Faltas' }));
    const tabela = await screen.findByRole('table', { name: 'Prévia de Faltas' });
    expect(within(tabela).getByText('PAC-2026-000193')).toBeInTheDocument();
    expect(within(tabela).getByText('TUSS 40901173')).toBeInTheDocument();
    expect(screen.getByText('Este relatório tem dado de saúde.')).toBeInTheDocument();
  });

  it('o faturamento vê só os relatórios dele', async () => {
    estado.sessao = SESSOES.faturista;
    renderizar('/relatorios');
    const catalogo = await screen.findByRole('navigation', { name: 'Catálogo de relatórios' });
    expect(within(catalogo).getAllByRole('button').map((b) => b.textContent)).toEqual(['Produção por convênio', 'Glosas']);
  });

  it('exporta o CSV com o código e o SHA-256 do servidor', async () => {
    const usuario = userEvent.setup();
    const { salvarArquivo } = renderizar('/relatorios');
    await usuario.click(await screen.findByRole('button', { name: 'Exportar CSV' }));
    expect(await screen.findByText(new RegExp(`Registrado no lastro como EXP-2026-000001, SHA-256 ${SHA_FICTICIO}`))).toBeInTheDocument();
    expect(salvarArquivo).toHaveBeenCalledTimes(1);
    const pedido = pedidosDe('exportacao').at(-1);
    expect(pedido?.csrf).toBe(estado.sessao.csrf);
  });

  it('sem reautenticação recente, pede a confirmação e não baixa nada', async () => {
    fase1.exigirReautenticacao.add('exportar-relatorio');
    const usuario = userEvent.setup();
    const { salvarArquivo } = renderizar('/relatorios');
    await usuario.click(await screen.findByRole('button', { name: 'Exportar CSV' }));
    expect(await screen.findByText('Confirme sua senha e o segundo fator para exportar o relatório.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Confirmar identidade' })).toBeInTheDocument();
    expect(salvarArquivo).not.toHaveBeenCalled();
  });

  it('perfil sem relatório vê a orientação, sem catálogo', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/relatorios');
    expect(await screen.findByText('Seu perfil não tem relatórios.')).toBeInTheDocument();
    expect(screen.queryByRole('navigation', { name: 'Catálogo de relatórios' })).not.toBeInTheDocument();
  });
});
