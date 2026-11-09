import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { etiquetasSimuladas } from '../../../testes/nucleo-etiquetas';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Etiquetas e rastreio (Etiquetas.dc.html)', () => {
  it('mostra os contadores, o alerta da sala errada com dois identificadores e o lastro da etiqueta', async () => {
    const { container } = renderizar('/etiquetas');
    const alerta = await screen.findByRole('alert');
    expect(alerta).toHaveTextContent('Exame bloqueado na Sala 1.');
    expect(alerta).toHaveTextContent('A pulseira de Juliana Sampaio Rocha (12/04/1993, USG morfológica, Sala 2) foi bipada na Sala 1');
    expect(screen.getByText('impressas hoje')).toBeInTheDocument();
    const lastro = screen.getByRole('complementary', { name: 'Lastro da etiqueta' });
    expect(await within(lastro).findByText('Bipada na sala errada (Sala 1): exame bloqueado, nada enviado ao aparelho')).toBeInTheDocument();
    expect(within(lastro).getByText('Nasc. 12/04/1993')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('técnico encaminha à sala certa e a tela diz o que foi feito', async () => {
    estado.sessao = SESSOES.tecnico;
    const usuario = userEvent.setup();
    renderizar('/etiquetas');
    await usuario.click(await screen.findByRole('button', { name: 'Encaminhar à Sala 2' }));
    expect(await screen.findByText('Juliana Sampaio Rocha: divergência resolvida.')).toBeInTheDocument();
    expect(screen.getByText(/Paciente encaminhado à Sala 2\. Registrado no lastro da etiqueta ETQ-2026-000002/)).toBeInTheDocument();
    expect(etiquetasSimuladas.pedidos).toEqual([{ rota: 'resolver', corpo: { resolucao: 'encaminhado' } }]);
    // O técnico não reimprime nem invalida (só a recepção).
    expect(screen.queryByRole('button', { name: 'Invalidar' })).not.toBeInTheDocument();
  });

  it('recepção invalida com motivo obrigatório', async () => {
    const usuario = userEvent.setup();
    renderizar('/etiquetas');
    const lista = await screen.findByRole('region', { name: 'Etiquetas de hoje' });
    await usuario.click(await within(lista).findByRole('button', { name: 'ETQ-2026-000001, Maria Helena Duarte, nascimento 12/03/1968' }));
    const lastro = screen.getByRole('complementary', { name: 'Lastro da etiqueta' });
    await usuario.click(await within(lastro).findByRole('button', { name: 'Invalidar' }));
    const form = within(lastro).getByRole('form', { name: 'Invalidar ETQ-2026-000001' });
    expect(within(form).getByText('Invalidar a pulseira de Maria Helena Duarte, 12/03/1968')).toBeInTheDocument();
    await usuario.click(within(form).getByRole('button', { name: 'Confirmar' }));
    expect(within(form).getByText('Escolha ou escreva o motivo.')).toBeInTheDocument();
    expect(etiquetasSimuladas.pedidos).toHaveLength(0);
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'paciente desistiu do exame');
    await usuario.click(within(form).getByRole('button', { name: 'Confirmar' }));
    expect(await within(lastro).findByText('Etiqueta ETQ-2026-000001 invalidada.')).toBeInTheDocument();
    expect(etiquetasSimuladas.pedidos).toEqual([{ rota: 'invalidar', corpo: { motivo: 'paciente desistiu do exame' } }]);
  });

  it('caixa recebe a recusa do servidor', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/etiquetas');
    expect(await screen.findByText(/Seu perfil não pode ver as etiquetas/)).toBeInTheDocument();
  });
});
