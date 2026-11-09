import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Corpo clínico (Corpo-Clinico.dc.html)', () => {
  it('a segunda pessoa confere o CRM pedido por outra e o cadastro passa a valer', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/corpo-clinico');
    expect(await screen.findByText('3 médicos na unidade, 1 aguardando conferência')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /Dra\. Cecília Souto/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do médico' });
    expect(within(detalhe).getByText('Aguarda segunda conferência: registro profissional CRM-SP 654321, Radiologia.')).toBeInTheDocument();
    await usuario.click(within(detalhe).getByRole('button', { name: 'Conferi: aplicar' }));
    expect(within(detalhe).getByText('Diga o que conferiu (5 letras ou mais).')).toBeInTheDocument();
    expect(pedidosDe('decidir-corpo-clinico')).toHaveLength(0);
    await usuario.type(within(detalhe).getByRole('textbox', { name: 'Motivo da conferência' }), 'conferido no CFM');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Conferi: aplicar' }));
    expect(await within(detalhe).findByText('CRM-SP 654321, Radiologia')).toBeInTheDocument();
    expect(pedidosDe('decidir-corpo-clinico')[0]?.corpo).toEqual({
      aprovar: true,
      motivo: 'conferido no CFM',
    });
  });

  it('pede a mudança do registro com motivo; CRM de outro médico é recusado pelo servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/corpo-clinico');
    await usuario.click(await screen.findByRole('button', { name: /Dr\. Marcos Freire/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do médico' });
    expect(within(detalhe).getByText('Registro profissional não cadastrado.')).toBeInTheDocument();
    const form = within(detalhe).getByRole('form', {
      name: 'Registro profissional',
    });
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para conferência' }));
    expect(within(form).getByText('Escolha a UF.')).toBeInTheDocument();
    expect(pedidosDe('pedir-corpo-clinico')).toHaveLength(0);
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'UF do CRM' }), 'SP');
    await usuario.type(within(form).getByRole('textbox', { name: 'Número do CRM' }), '123456');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo do pedido' }), 'contratação do médico');
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para conferência' }));
    await esperarAlertaDoServidor('Este CRM já está no cadastro de outro médico desta unidade.', 'crm-em-uso');
    await usuario.clear(within(form).getByRole('textbox', { name: 'Número do CRM' }));
    await usuario.type(within(form).getByRole('textbox', { name: 'Número do CRM' }), '223344');
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para conferência' }));
    expect(await within(form).findByText('Pedido enviado para a conferência.')).toBeInTheDocument();
    const pedido = pedidosDe('pedir-corpo-clinico')[1];
    expect(pedido?.corpo).toEqual({
      tipo: 'registro',
      registro: { uf: 'SP', numero: '223344', rqe: null, especialidade: null },
      motivo: 'contratação do médico',
    });
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.chave).not.toBe(pedidosDe('pedir-corpo-clinico')[0]?.chave);
    // Quem pediu não confere o próprio pedido.
    expect(await within(detalhe).findByText('Quem pediu não confere. Peça a outra pessoa da gerência.')).toBeInTheDocument();
  });

  it('pedir sem reautenticação recente leva à confirmação de identidade', async () => {
    fase1.exigirReautenticacao.add('pedir-corpo-clinico');
    const usuario = userEvent.setup();
    renderizar('/corpo-clinico');
    await usuario.click(await screen.findByRole('button', { name: /Dr\. Paulo Medeiros/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do médico' });
    await usuario.click(within(detalhe).getByRole('tab', { name: 'Realiza e lauda' }));
    const form = within(detalhe).getByRole('form', { name: 'Realiza e lauda' });
    await usuario.click(within(within(form).getByRole('group', { name: 'Realiza na sala' })).getByRole('checkbox', { name: 'Tomografia' }));
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo do pedido' }), 'passou a laudar tomografia');
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para conferência' }));
    expect(await within(form).findByText('Confirme sua senha e o segundo fator para pedir a mudança do que realiza e lauda.')).toBeInTheDocument();
    expect(pedidosDe('pedir-corpo-clinico')[0]?.corpo).toMatchObject({
      tipo: 'habilitacao',
      habilitacao: { realiza: ['US', 'CT'], lauda: ['US'] },
    });
  });

  it('a recepção não administra o corpo clínico', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/corpo-clinico');
    expect(await screen.findByText('Você não administra o corpo clínico')).toBeInTheDocument();
  });
});
