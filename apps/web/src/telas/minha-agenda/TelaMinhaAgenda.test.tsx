import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { gestaoSimulada } from '../../../testes/nucleo-gestao';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.medico;
});

describe('Minha agenda (Agenda-Medico.dc.html)', () => {
  it('mostra os pacientes do médico com nome e nascimento, em grupos, e os números do dia', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/minha-agenda');
    const tabela = await screen.findByRole('table', { name: 'Pacientes do dia' });
    expect(within(tabela).getByText('Antônio Carlos Ribeiro')).toBeInTheDocument();
    expect(within(tabela).getByText('nascimento 15/06/1954')).toBeInTheDocument();
    expect(within(tabela).getByText(/Sala 1, senha P012/)).toBeInTheDocument();
    expect(within(tabela).getByRole('link', { name: 'Laudar' })).toHaveAttribute('href', '/laudos/00000010-0000-7000-8000-000000000001');
    expect(screen.getByText('2 vencem hoje')).toBeInTheDocument();
    await usuario.click(screen.getByRole('tab', { name: 'Esperando 1' }));
    const filtrada = screen.getByRole('table', { name: 'Pacientes do dia' });
    expect(within(filtrada).getAllByRole('row')).toHaveLength(2);
    expect(within(filtrada).queryByText('Cláudia Bastos Nunes')).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('pede o bloqueio da agenda com motivo e diz que só vale depois da aprovação', async () => {
    const usuario = userEvent.setup();
    renderizar('/minha-agenda');
    await usuario.click(await screen.findByRole('button', { name: 'Bloquear horário' }));
    const form = screen.getByRole('form', { name: 'Pedido de bloqueio' });
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para aprovação' }));
    expect(within(form).getByText('Escolha o motivo para enviar.')).toBeInTheDocument();
    expect(pedidosDe('ausencia')).toHaveLength(0);
    await digitarData(usuario, 'Primeiro dia', '20122026', form);
    await digitarData(usuario, 'Último dia', '22122026', form);
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Motivo' }), 'Congresso ou curso');
    await usuario.click(within(form).getByRole('button', { name: 'Enviar para aprovação' }));
    expect(await screen.findByText('Pedido AUS-2026-000043 enviado à coordenação.')).toBeInTheDocument();
    expect(screen.getByText(/Enquanto não for aprovado, a agenda continua aberta\./)).toBeInTheDocument();
    const pedido = pedidosDe('ausencia').at(-1);
    expect(pedido?.chave).toMatch(/^[0-9a-f-]{36}$/);
    expect(pedido?.corpo).toMatchObject({ inicio: '2026-12-20T00:00:00-03:00', fim: '2026-12-23T00:00:00-03:00', motivo: 'congresso' });
    expect(gestaoSimulada.ausencias).toHaveLength(2);
  });

  it('quem não é médico vê que a agenda é do médico executante', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/minha-agenda');
    expect(await screen.findByText('Esta agenda é do médico executante.')).toBeInTheDocument();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });
});

describe('Ausências dos médicos', () => {
  it('a gerência aprova o pedido; recusar exige motivo', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    const { container } = renderizar('/ausencias-medicas');
    expect(await screen.findByText('Dr. Paulo Medeiros')).toBeInTheDocument();
    expect(screen.getByText(/14 paciente\(s\) agendado\(s\) no período\./)).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Recusar' }));
    await usuario.click(screen.getByRole('button', { name: 'Recusar o pedido' }));
    expect(screen.getByText('Diga o motivo ao médico (5 letras ou mais).')).toBeInTheDocument();
    expect(pedidosDe('decisao-ausencia')).toHaveLength(0);
    await usuario.click(screen.getByRole('button', { name: 'Voltar' }));
    await usuario.click(screen.getByRole('button', { name: 'Aprovar o bloqueio' }));
    await usuario.click(await screen.findByRole('tab', { name: 'Aprovadas 1' }));
    expect(await screen.findByText(/Decidido por Patrícia Lemos\./)).toBeInTheDocument();
    expect(pedidosDe('decisao-ausencia').at(-1)?.corpo).toEqual({ aprovar: true });
  });

  it('a recepção acompanha sem botão de decisão', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar('/ausencias-medicas');
    expect(await screen.findByText('Dr. Paulo Medeiros')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Aprovar o bloqueio' })).not.toBeInTheDocument();
  });
});
