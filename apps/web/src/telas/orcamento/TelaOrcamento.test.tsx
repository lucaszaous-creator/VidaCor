import { screen, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

async function montar(usuario: UserEvent) {
  await usuario.type(await screen.findByRole('searchbox', { name: 'Para quem' }), 'Vera');
  await usuario.click(await screen.findByRole('button', { name: /^Vera Lúcia Antunes, nascimento 18\/09\/1957/ }));
  for (const nome of ['cardiograma', 'carótidas']) {
    await usuario.type(screen.getByRole('searchbox', { name: 'Buscar exame para adicionar' }), nome);
    await usuario.click((await screen.findAllByRole('button', { name: /^Adicionar / }))[0] as HTMLElement);
  }
}

describe('Orçamento particular (Orcamento.dc.html)', () => {
  it('oferece o pacote, aplica desconto com motivo e mostra o total calculado pelo servidor', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/orcamentos');
    await montar(usuario);
    expect(within(screen.getByRole('list', { name: 'Exames do orçamento' })).getAllByRole('listitem')).toHaveLength(2);
    await usuario.click(screen.getByRole('button', { name: 'Aplicar pacote' }));
    await usuario.type(screen.getByRole('textbox', { name: 'Desconto (%)' }), '5');
    expect(screen.getByText('Sem motivo o desconto não vale.')).toBeInTheDocument();
    await usuario.selectOptions(screen.getByRole('combobox', { name: 'Motivo do desconto' }), 'paciente_frequente');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Emitir orçamento' }));
    expect(await screen.findByText('Orçamento ORC-2026-000001 emitido para Vera Lúcia Antunes.')).toBeInTheDocument();
    expect(screen.getByText(/Total R\$\s513,00 pela tabela v12 e regra de pacote v3\./)).toBeInTheDocument();
    // A tela só pede: nada de preço ou total no corpo.
    const corpo = pedidosDe('emitir-orcamento')[0]?.corpo as Record<string, unknown>;
    expect(corpo).toEqual({ paciente_id: expect.any(String) as string, itens: ['40901114', '40901300'], pacote: 'PAC-CARDIO', desconto: { pontos_base: 500, motivo: 'paciente_frequente' }, pagamento_previsto: 'pix' });
  });

  it('desconto acima do limite não é enviado e diz o que fazer', async () => {
    const usuario = userEvent.setup();
    renderizar('/orcamentos');
    await montar(usuario);
    await usuario.type(screen.getByRole('textbox', { name: 'Desconto (%)' }), '15');
    expect(screen.getByText(/Passa do limite de .* do orçamento/)).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: 'Emitir orçamento' }));
    expect(pedidosDe('emitir-orcamento')).toHaveLength(0);
  });

  it('sem paciente nem exame, pede antes de enviar', async () => {
    const usuario = userEvent.setup();
    renderizar('/orcamentos');
    await usuario.click(await screen.findByRole('button', { name: 'Emitir orçamento' }));
    expect(screen.getByText('Escolha o paciente do cadastro.')).toBeInTheDocument();
    expect(screen.getByText('Inclua ao menos um exame.')).toBeInTheDocument();
    expect(pedidosDe('emitir-orcamento')).toHaveLength(0);
  });
});
