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
  estado.sessao = SESSOES.ti;
});

describe('Aparelhos (Aparelhos.dc.html)', () => {
  it('mostra a situação e a atividade em números e desativa com motivo', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/aparelhos');
    expect(await screen.findByText('41 estudos recebidos hoje, 1 sem paciente conciliado')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /Revolution CT/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do aparelho' });
    expect(within(detalhe).getByText('2 lista(s) de trabalho com erro de envio.')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /Vivid E95/ }));
    await usuario.click(within(detalhe).getByRole('button', { name: 'Desativar aparelho' }));
    expect(within(detalhe).getByText('Escreva o motivo (5 letras ou mais).')).toBeInTheDocument();
    expect(pedidosDe('mudar-aparelho')).toHaveLength(0);
    await usuario.type(within(detalhe).getByRole('textbox', { name: 'Motivo para desativar' }), 'manutenção do transdutor');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Desativar aparelho' }));
    expect(await within(detalhe).findByText('Vivid E95 desativado.')).toBeInTheDocument();
    expect(pedidosDe('mudar-aparelho')[0]?.corpo).toEqual({ tipo: 'desativar', motivo: 'manutenção do transdutor' });
  });

  it('cadastro com AE Title em uso é recusado pelo servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/aparelhos');
    await usuario.click(await screen.findByRole('button', { name: 'Adicionar aparelho' }));
    const form = screen.getByRole('form', { name: 'Adicionar aparelho' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Nome do aparelho' }), 'Logiq E10');
    await usuario.type(within(form).getByRole('textbox', { name: 'AE Title' }), 'sala1');
    await usuario.type(within(form).getByRole('textbox', { name: 'Modalidade' }), 'us');
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Sala' }), within(form).getAllByRole('option')[1] as HTMLOptionElement);
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'aparelho novo');
    await usuario.click(within(form).getByRole('button', { name: 'Cadastrar aparelho' }));
    await esperarAlertaDoServidor('Este AE Title já é de outro aparelho.', 'ae-title-em-uso');
    expect(pedidosDe('cadastrar-aparelho')[0]?.corpo).toMatchObject({ ae_title: 'SALA1', modalidade: 'US' });
  });
});
