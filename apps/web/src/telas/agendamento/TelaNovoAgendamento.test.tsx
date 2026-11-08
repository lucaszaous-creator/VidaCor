import { screen, waitFor, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { ANTONIO, GUIA_UNIMED, SALA_1 } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

async function prepararAgendamento(usuario: UserEvent): Promise<void> {
  await usuario.type(await screen.findByRole('searchbox', { name: 'Buscar paciente por nome' }), 'Antônio');
  expect(await screen.findByRole('button', { name: 'Antônio Carlos Ribeiro, nascimento 02/01/1980' })).toBeInTheDocument();
  await usuario.click(screen.getByRole('button', { name: 'Antônio Carlos Ribeiro, nascimento 15/06/1954' }));
  await usuario.selectOptions(screen.getByRole('combobox', { name: 'Sala' }), SALA_1.id);
  await digitarData(usuario, 'Dia do exame', '07112026');
  await usuario.click(await screen.findByRole('button', { name: '09:00' }));
  await usuario.type(screen.getByRole('textbox', { name: 'Código TUSS 1' }), '40901114');
  await usuario.click(screen.getByRole('button', { name: 'Adicionar procedimento' }));
  await usuario.type(screen.getByRole('textbox', { name: 'Código TUSS 2' }), '40901300');
}

async function registrarGuia(usuario: UserEvent): Promise<void> {
  const secao = screen.getByRole('region', { name: 'Convênio e guia' });
  await usuario.selectOptions(within(secao).getByRole('combobox', { name: 'Operadora' }), GUIA_UNIMED.operadora);
  await usuario.type(within(secao).getByRole('textbox', { name: 'Nº da guia na operadora' }), GUIA_UNIMED.numero);
  await usuario.type(within(secao).getByRole('textbox', { name: 'Carteirinha' }), GUIA_UNIMED.carteirinha);
  await usuario.type(within(secao).getByRole('textbox', { name: 'Senha de autorização' }), GUIA_UNIMED.senha);
  await digitarData(usuario, 'Senha válida até', '20112026', secao);
  await usuario.type(within(secao).getByRole('textbox', { name: 'Médico solicitante (nome e CRM)' }), GUIA_UNIMED.solicitante);
  await usuario.click(within(secao).getByRole('button', { name: 'Registrar guia' }));
  expect(await within(secao).findByRole('checkbox', { name: /TUSS 40901114/ })).toBeChecked();
}

describe('Agendamento de convênio com guia (FC-AGD)', () => {
  it('caminho feliz: guia registrada com saldo do servidor e agendamento confirmado', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/agendamentos/novo');
    await prepararAgendamento(usuario);
    expect(screen.getByRole('button', { name: '09:00' })).toHaveAttribute('aria-pressed', 'true');
    await registrarGuia(usuario);
    expect(screen.getByRole('region', { name: 'Convênio e guia' })).toHaveTextContent('Saldo conferido pelo servidor: 1');
    expect(pedidosDe('guia')[0]?.corpo).toEqual({
      operadora: GUIA_UNIMED.operadora,
      numero: 'UN-55120',
      carteirinha: GUIA_UNIMED.carteirinha,
      senha: 'UN-88231-X',
      validade: '2026-11-20',
      paciente_id: ANTONIO.paciente.id,
      solicitante: GUIA_UNIMED.solicitante,
      procedimentos: [
        { tuss: '40901114', quantidade: 1 },
        { tuss: '40901300', quantidade: 1 },
      ],
    });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);

    await usuario.click(screen.getByRole('button', { name: 'Agendar com a guia' }));
    const feito = await screen.findByText(/Agendamento confirmado\./);
    expect(feito.closest('[role="status"]')).toHaveTextContent('Antônio Carlos Ribeiro, nascimento 15/06/1954, 07/11/2026 09:00, Sala 1, eco.');
    const pedido = pedidosDe('agendamento')[0];
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo).toEqual({
      paciente_id: ANTONIO.paciente.id,
      sala_id: SALA_1.id,
      inicio: '2026-11-07T09:00:00-03:00',
      procedimentos: ['40901114', '40901300'],
      guia_id: expect.stringMatching(UUID) as unknown,
      reserva_provisoria: false,
    });
  });

  it('horário ocupado (409) mostra os próximos horários livres clicáveis e o novo envio usa outra chave', async () => {
    servidor.use(
      http.post('*/v1/agendamentos', async ({ request }) => {
        const corpo = (await request.clone().json()) as { inicio: string };
        if (corpo.inicio.includes('T09:00')) {
          return problema(409, 'horario-ocupado', 'O horário das 09:00 da Sala 1 acabou de ser ocupado. Nada foi agendado. Escolha um dos próximos horários livres.', {
            sugestoes: ['2026-11-07T09:30:00-03:00', '2026-11-07T10:00:00-03:00', '2026-11-07T10:30:00-03:00'],
          });
        }
        return undefined;
      }),
    );
    const usuario = userEvent.setup();
    renderizar('/agendamentos/novo');
    await prepararAgendamento(usuario);
    await registrarGuia(usuario);
    await usuario.click(screen.getByRole('button', { name: 'Agendar com a guia' }));
    const alerta = await esperarAlertaDoServidor('O horário das 09:00 da Sala 1 acabou de ser ocupado.', 'horario-ocupado');
    const sugestoes = within(alerta).getByRole('group', { name: 'Próximos horários livres' });
    expect(within(sugestoes).getAllByRole('button').map((b) => b.textContent)).toEqual(['Usar 09:30', 'Usar 10:00', 'Usar 10:30']);
    await usuario.click(within(sugestoes).getByRole('button', { name: 'Usar 09:30' }));
    await waitFor(() => {
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: '09:30' })).toHaveAttribute('aria-pressed', 'true');
    await usuario.click(screen.getByRole('button', { name: 'Agendar com a guia' }));
    expect(await screen.findByText(/Agendamento confirmado\./)).toBeInTheDocument();
    expect(pedidosDe('agendamento').at(-1)?.corpo).toMatchObject({ inicio: '2026-11-07T09:30:00-03:00' });
  });

  it.each([
    [409, 'guia-em-uso', 'A guia UN-55120 já está em uso. Nada foi agendado. É preciso outra guia.', { atendimento_codigo: 'ATD-2026-118790' }, 'Guia em uso no atendimento ATD-2026-118790: registre outra guia.'],
    [422, 'procedimento-nao-autorizado', 'O procedimento não está autorizado na guia. Nada foi agendado.', { tuss: '40901220' }, 'Procedimento TUSS 40901220 fora da guia: peça autorização à operadora.'],
    [422, 'senha-vencida', 'A senha da guia vence antes da data do exame. Nada foi agendado.', { validade: '2026-11-20' }, 'A senha vale até 20/11/2026: remarque antes disso ou peça prorrogação.'],
    [422, 'saldo-esgotado', 'O saldo do procedimento 40901114 na guia é zero. Nada foi agendado. Peça nova autorização.', {}, ''],
    [403, 'sem-permissao', 'Seu perfil não pode agendar. Nada foi feito.', {}, ''],
  ])('erro %i %s: mensagem do servidor com o que fazer', async (status, codigo, detalhe, extras, complemento) => {
    servidor.use(http.post('*/v1/agendamentos', () => problema(status, codigo, detalhe, extras)));
    const usuario = userEvent.setup();
    renderizar('/agendamentos/novo');
    await prepararAgendamento(usuario);
    await registrarGuia(usuario);
    await usuario.click(screen.getByRole('button', { name: 'Agendar com a guia' }));
    const alerta = await esperarAlertaDoServidor(detalhe, codigo);
    if (complemento !== '') expect(alerta).toHaveTextContent(complemento);
    if (codigo === 'guia-em-uso') {
      await usuario.click(within(alerta).getByRole('button', { name: 'Registrar outra guia' }));
      expect(await screen.findByRole('button', { name: 'Registrar guia' })).toBeInTheDocument();
    }
    expect(screen.queryByText(/Agendamento confirmado\./)).not.toBeInTheDocument();
  });

  it('sem guia, só reserva provisória com prazo; a guia entregue depois confirma a reserva', async () => {
    const usuario = userEvent.setup();
    renderizar('/agendamentos/novo');
    await prepararAgendamento(usuario);
    expect(screen.getByRole('button', { name: 'Agendar com a guia' })).toBeDisabled();
    await usuario.click(screen.getByRole('button', { name: 'Reservar sem guia' }));
    const reserva = await screen.findByText(/Reserva provisória feita\./);
    expect(reserva.closest('[role="status"]')).toHaveTextContent('O horário fica reservado até 09/11/2026 09:16.');
    expect(pedidosDe('agendamento')[0]?.corpo).toMatchObject({ guia_id: null, reserva_provisoria: true, procedimentos: ['40901114', '40901300'] });

    await registrarGuia(usuario);
    await usuario.click(screen.getByRole('button', { name: 'Confirmar a reserva com a guia' }));
    expect(await screen.findByText(/Agendamento confirmado\./)).toBeInTheDocument();
  });

  it('a tela pede o que falta antes de enviar', async () => {
    const usuario = userEvent.setup();
    renderizar('/agendamentos/novo');
    await usuario.click(await screen.findByRole('button', { name: 'Reservar sem guia' }));
    expect(screen.getByText('Escolha o paciente.')).toBeInTheDocument();
    expect(screen.getByText('Escolha a sala.')).toBeInTheDocument();
    expect(screen.getByText('O código TUSS tem 8 dígitos.')).toBeInTheDocument();
    expect(pedidosDe('agendamento')).toHaveLength(0);
  });
});
