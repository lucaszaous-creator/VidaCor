import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
});

describe('Qualidade (Qualidade.dc.html)', () => {
  it('técnico notifica evento grave anônimo e vê o prazo da Anvisa, sem acesso ao painel', async () => {
    estado.sessao = SESSOES.tecnico;
    const usuario = userEvent.setup();
    const { container } = renderizar('/qualidade');
    const form = await screen.findByRole('form', { name: 'Notificar evento ou quase-erro' });
    expect(screen.queryByRole('region', { name: 'Registros da qualidade' })).not.toBeInTheDocument();
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'Gravidade do dano' }), 'grave');
    await usuario.type(within(form).getByRole('textbox', { name: 'Descrição do evento' }), 'Reação anafilática após contraste.');
    await usuario.click(within(form).getByRole('checkbox', { name: /Notificar de forma anônima/ }));
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(form).getByRole('button', { name: 'Enviar notificação' }));
    expect(await screen.findByText(/Dano grave: o Núcleo tem até/)).toBeInTheDocument();
    expect(pedidosDe('notificar-evento')[0]?.corpo).toMatchObject({ gravidade: 'grave', anonimo: true, quase_erro: false });
  });

  it('gerência vê o alerta da Anvisa e registra andamento com a versão vista', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    renderizar('/qualidade');
    expect(await screen.findByText('1 evento(s) grave(s) aguardando notificação à Anvisa.')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /Nobreak da sala 6/ }));
    const d = screen.getByRole('region', { name: 'Detalhe da qualidade' });
    await usuario.type(within(d).getByRole('textbox', { name: 'Andamento' }), 'Chamado aberto com o fornecedor.');
    await usuario.click(within(d).getByRole('button', { name: 'Registrar andamento' }));
    expect(await within(d).findByText('Andamento registrado.')).toBeInTheDocument();
    expect(pedidosDe('andamento-nc')[0]?.corpo).toMatchObject({ situacao: 'em_acao', versao: 1 });
  });
});
