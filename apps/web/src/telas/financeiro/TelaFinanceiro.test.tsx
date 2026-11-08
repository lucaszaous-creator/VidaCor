import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Financeiro (Financeiro.dc.html)', () => {
  it('resumo, título aguardando aprovação e aprovação pela gerência', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/financeiro');
    expect(await screen.findByRole('region', { name: 'Resumo do mês' })).toHaveTextContent('R$ 538.900,00');
    const tabela = await screen.findByRole('grid', { name: 'Títulos a pagar' });
    expect(within(tabela).getByText('Aprovação 1 de 2')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(tabela).getByRole('row', { name: /Distribuidora Fictícia/ }));
    await usuario.click(await screen.findByRole('button', { name: 'Aprovar' }));
    expect(await screen.findByText('Aprovação registrada.')).toBeInTheDocument();
    expect(pedidosDe('aprovar-titulo')).toHaveLength(1);
  });

  it('novo lançamento: o servidor diz quantas aprovações precisa', async () => {
    const usuario = userEvent.setup();
    renderizar('/financeiro');
    const form = await screen.findByRole('region', { name: 'Novo lançamento' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Para quem' }), 'Gráfica Fictícia');
    await usuario.type(within(form).getByRole('textbox', { name: 'Descrição' }), 'Pulseiras, NF 88');
    await usuario.type(within(form).getByRole('textbox', { name: 'Valor (R$)' }), '12.500,00');
    await digitarData(usuario, 'Vencimento', '30112026');
    await usuario.click(within(form).getByRole('button', { name: 'Lançar' }));
    expect(await within(form).findByText(/Precisa de 2 aprovação\(ões\) de outra pessoa/)).toBeInTheDocument();
    expect(pedidosDe('lancar-titulo')[0]?.corpo).toMatchObject({ tipo: 'pagar', valor_centavos: 1250000, vencimento: '2026-11-30', categoria: 'fornecedor' });
  });

  it('repasse mostra quem está sem regra e gera os títulos', async () => {
    const usuario = userEvent.setup();
    renderizar('/financeiro');
    await usuario.click(await screen.findByRole('tab', { name: 'Repasse' }));
    const tabela = await screen.findByRole('grid', { name: 'Repasse por médico' });
    expect(within(tabela).getByText('Sem regra')).toBeInTheDocument();
    expect(within(tabela).getByText('R$ 7.200,00')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: 'Gerar títulos do repasse' }));
    expect(await screen.findByText(/Sem regra de repasse, nada foi gerado para: Dr\. Marcos Freire\./)).toBeInTheDocument();
  });

  it('recepção não acessa o financeiro', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/financeiro');
    expect(await screen.findByText('Você não acessa o financeiro')).toBeInTheDocument();
  });
});
