import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { comunicacaoDoPainel } from '../../../testes/nucleo-comunicacao';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';
import { amanhaNaClinica } from './TelaComunicacao';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Comunicação com pacientes (Comunicacao.dc.html)', () => {
  it('mostra o registro com dois identificadores, situação e resposta', async () => {
    const { container } = renderizar('/comunicacao');
    expect(await screen.findByText('Antônio Carlos Ribeiro')).toBeInTheDocument();
    expect(screen.getByText('15/06/1954')).toBeInTheDocument();
    expect(screen.getByText('Confirmou')).toBeInTheDocument();
    expect(screen.getByText('Não enviada: sem celular')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('filtra as não entregues no servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/comunicacao');
    await screen.findByText('Antônio Carlos Ribeiro');
    await usuario.click(screen.getByRole('button', { name: 'Não entregues' }));
    expect(await screen.findByText('Maria Helena Duarte')).toBeInTheDocument();
    expect(screen.queryByText('Antônio Carlos Ribeiro')).not.toBeInTheDocument();
  });

  it('a recepção pede a confirmação de amanhã e vê quantas saíram', async () => {
    const usuario = userEvent.setup();
    renderizar('/comunicacao');
    await usuario.click(await screen.findByRole('button', { name: 'Pedir confirmação de amanhã' }));
    expect(await screen.findByText(/confirmações pedidas para/)).toBeInTheDocument();
    expect(pedidosDe('lembretes')[0]?.corpo).toEqual({ data: amanhaNaClinica() });
  });

  it('a gerência desliga um aviso com motivo e a versão vista', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    renderizar('/comunicacao');
    await usuario.click(await screen.findByRole('tab', { name: 'Mensagens automáticas' }));
    const aviso = screen.getByRole('listitem', { name: 'Confirmação do horário' });
    await usuario.click(within(aviso).getByRole('button', { name: 'Desligar' }));
    const form = within(aviso).getByRole('form', { name: 'Desligar Confirmação do horário' });
    await usuario.click(within(form).getByRole('button', { name: 'Desligar aviso' }));
    expect(within(form).getByText('Escreva o motivo (5 letras ou mais).')).toBeInTheDocument();
    expect(comunicacaoDoPainel.mudancas).toHaveLength(0);
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'Troca do número do WhatsApp');
    await usuario.click(within(form).getByRole('button', { name: 'Desligar aviso' }));
    expect(await screen.findByText('Aviso desligado.')).toBeInTheDocument();
    expect(comunicacaoDoPainel.mudancas).toEqual([{ ativo: false, motivo: 'Troca do número do WhatsApp', versao_vista: null }]);
  });

  it('a recepção vê os avisos mas não liga nem desliga', async () => {
    const usuario = userEvent.setup();
    renderizar('/comunicacao');
    await usuario.click(await screen.findByRole('tab', { name: 'Mensagens automáticas' }));
    expect(screen.getByText(/Desligado por Patrícia Lemos/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Desligar' })).not.toBeInTheDocument();
    expect(screen.getByText(/Só a gerência liga e desliga/)).toBeInTheDocument();
  });

  it('caixa recebe a recusa do servidor', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/comunicacao');
    expect(await screen.findByText(/Seu perfil não pode ver a comunicação com pacientes/)).toBeInTheDocument();
  });

  it('amanhã no fuso da clínica', () => {
    expect(amanhaNaClinica(new Date('2026-11-09T02:30:00Z'))).toBe('2026-11-09');
    expect(amanhaNaClinica(new Date('2026-11-09T15:00:00Z'))).toBe('2026-11-10');
  });
});
