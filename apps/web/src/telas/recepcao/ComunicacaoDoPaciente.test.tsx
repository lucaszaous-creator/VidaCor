import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('WhatsApp do paciente na recepção', () => {
  it('mostra o histórico e desliga os avisos com motivo', async () => {
    const usuario = userEvent.setup();
    renderizar('/recepcao?data=2026-11-06');
    await usuario.click(await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento/ }));
    const bloco = await screen.findByRole('region', { name: 'WhatsApp do paciente' });
    expect(bloco).toHaveTextContent('Avisos ligados');
    expect(bloco).toHaveTextContent('Confirmação de horário');
    expect(bloco).toHaveTextContent('paciente confirmou');
    await usuario.click(within(bloco).getByRole('button', { name: 'Desligar avisos' }));
    expect(within(bloco).getByText('Diga o motivo.')).toBeInTheDocument();
    await usuario.type(within(bloco).getByRole('textbox', { name: 'Motivo da troca' }), 'pedido no balcão');
    await usuario.click(within(bloco).getByRole('button', { name: 'Desligar avisos' }));
    expect(await within(bloco).findByText('Avisos desligados')).toBeInTheDocument();
    expect(pedidosDe('preferencia-whatsapp')[0]?.corpo).toEqual({ whatsapp: false, motivo: 'pedido no balcão' });
  });

  it('a agenda pede a confirmação dos horários por WhatsApp', async () => {
    const usuario = userEvent.setup();
    renderizar('/agenda?data=2099-01-05');
    await usuario.click(await screen.findByRole('button', { name: 'Confirmar por WhatsApp' }));
    expect(await screen.findByText('7 confirmação(ões) a caminho.')).toBeInTheDocument();
    expect(pedidosDe('lembretes')[0]?.corpo).toEqual({ data: '2099-01-05' });
  });
});
