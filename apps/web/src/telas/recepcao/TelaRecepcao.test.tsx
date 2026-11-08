import { screen, waitFor, within } from '@testing-library/react';
import userEvent, { type UserEvent } from '@testing-library/user-event';
import { http } from 'msw';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor, UUID, voltarDa } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { ANTONIO, ATENDIMENTO_ANTONIO } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, problema, reiniciarNucleo, servidor } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = '/recepcao?data=2026-11-06';

beforeEach(() => {
  reiniciarNucleo();
});

async function abrirAntonio(usuario: UserEvent): Promise<HTMLElement> {
  await usuario.click(await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954, 09:00/ }));
  return screen.getByRole('region', { name: 'Paciente escolhido' });
}

async function preencherCheckin(usuario: UserEvent, painel: HTMLElement, nascimento = '15061954'): Promise<void> {
  await usuario.type(within(painel).getByRole('textbox', { name: 'Nome completo' }), 'Antônio Carlos Ribeiro');
  await digitarData(usuario, 'Data de nascimento', nascimento, painel);
  await usuario.click(within(painel).getByRole('checkbox', { name: /Documento com foto conferido/ }));
  await usuario.click(within(painel).getByRole('button', { name: 'Conferir e fazer check-in' }));
  const dialogo = await screen.findByRole('alertdialog', { name: 'Fazer o check-in de Antônio Carlos Ribeiro?' });
  expect(dialogo).toHaveTextContent('Antônio Carlos Ribeiro, nascimento 15/06/1954');
  await usuario.click(within(dialogo).getByRole('button', { name: 'Fazer check-in' }));
}

function comAtendimentoDoAntonio(): void {
  const item = estado.itens.find((i) => i.id === ANTONIO.id);
  if (item !== undefined) {
    item.atendimento_id = ATENDIMENTO_ANTONIO.id;
    item.situacao = 'em_atendimento';
  }
}

describe('Recepção: check-in com dois identificadores e pulseira (FC-CHK)', () => {
  it('caminho feliz: alerta de homônimo, conferência com documento, senha e pulseira só com nome, nascimento e códigos', async () => {
    const usuario = userEvent.setup();
    const { container, imprimir } = renderizar(URL);
    expect(await screen.findByText('Homônimo hoje: confira o nascimento')).toBeInTheDocument();
    const painel = await abrirAntonio(usuario);

    const homonimo = within(painel).getByText(/Homônimo hoje\./).closest('[role="status"]');
    expect(homonimo).toHaveTextContent('Atenção');
    expect(homonimo).toHaveTextContent('Antônio Carlos Ribeiro, nascido em 02/01/1980 também tem exame hoje.');
    expect(homonimo).toHaveTextContent('Este paciente nasceu em 15/06/1954');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);

    await preencherCheckin(usuario, painel);
    const feito = await within(painel).findByText(/Check-in feito\./);
    expect(feito.closest('[role="status"]')).toHaveTextContent('Senha A012, atendimento ATD-2026-118791, accession ACC-2026-052311.');
    const pulseira = within(painel).getByRole('figure', { name: 'Pulseira ETQ-2026-018742, válida' });
    expect(pulseira).toHaveTextContent('ANTÔNIO CARLOS RIBEIRO');
    expect(pulseira).toHaveTextContent('Nasc. 15/06/1954');
    expect(pulseira).toHaveTextContent('ACC-2026-052311');
    expect(pulseira).not.toHaveTextContent('Doppler');

    await usuario.click(within(painel).getByRole('button', { name: 'Imprimir pulseira' }));
    expect(imprimir).toHaveBeenCalledTimes(1);

    const [pedido] = estado.pedidosDeCheckin;
    expect(pedido?.chave).toMatch(UUID);
    expect(pedido?.corpo.confirmacao).toEqual({ nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15', documento_com_foto: true });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('menu de contexto do paciente abre pelo teclado (Shift+F10), com nome e nascimento, sem inventar ações', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    const alvo = await screen.findByRole('button', { name: /^Antônio Carlos Ribeiro, nascimento 15\/06\/1954, 09:00/ });
    alvo.focus();
    await usuario.keyboard('{Shift>}{F10}{/Shift}');
    const menu = await screen.findByRole('menu', { name: 'Ações de Antônio Carlos Ribeiro' });
    expect(screen.getByText(/Nascimento 15\/06\/1954, 09:00, Eco e Doppler de carótidas/)).toBeInTheDocument();
    const itens = within(menu).getAllByRole('menuitem');
    expect(itens).toHaveLength(1);
    expect(itens[0]).toHaveAttribute('aria-disabled', 'true');
    expect(itens[0]).toHaveTextContent('Ações deste item ainda não vêm do servidor');
    await usuario.keyboard('{Escape}');
    await waitFor(() => {
      expect(alvo).toHaveFocus();
    });
  });

  it('sem documento com foto conferido, a tela pede antes de enviar', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    const painel = await abrirAntonio(usuario);
    await usuario.type(within(painel).getByRole('textbox', { name: 'Nome completo' }), 'Antônio Carlos Ribeiro');
    await digitarData(usuario, 'Data de nascimento', '15061954', painel);
    await usuario.click(within(painel).getByRole('button', { name: 'Conferir e fazer check-in' }));
    expect(within(painel).getByText('Confira o documento com foto do paciente antes do check-in.')).toBeInTheDocument();
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(estado.pedidosDeCheckin).toHaveLength(0);
  });

  it.each([
    [422, 'identificacao-incompleta', 'Falta conferir a data de nascimento. Nada foi gravado: nenhuma senha ou pulseira foi emitida.'],
    [422, 'identificacao-nao-confere', 'O nascimento informado não confere com o cadastro. Nada foi gravado. Corrija o cadastro com motivo ou identifique o paciente certo.'],
    [409, 'checkin-ja-realizado', 'O check-in deste agendamento já foi feito por Carla Mendes às 08:52. Nada foi gravado de novo.'],
    [409, 'situacao-nao-permite-checkin', 'O agendamento foi cancelado com motivo e não aceita check-in. Nada foi gravado.'],
    [403, 'sem-permissao', 'Seu perfil não faz check-in. Nada foi gravado. Peça a uma pessoa da recepção.'],
  ])('erro %i %s: mostra a mensagem do servidor, sem senha nem pulseira', async (status, codigo, detalhe) => {
    servidor.use(http.post('*/v1/atendimentos', () => problema(status, codigo, detalhe)));
    const usuario = userEvent.setup();
    renderizar(URL);
    const painel = await abrirAntonio(usuario);
    await preencherCheckin(usuario, painel);
    await esperarAlertaDoServidor(detalhe, codigo, painel);
    expect(within(painel).queryByRole('figure')).not.toBeInTheDocument();
    expect(within(painel).getByRole('textbox', { name: 'Nome completo' })).toHaveValue('Antônio Carlos Ribeiro');
  });
});

describe('Recepção: reimpressão e correção (FC-ETQ)', () => {
  it('reimpressão exige motivo, invalida a anterior e mostra as duas', async () => {
    comAtendimentoDoAntonio();
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    const painel = await abrirAntonio(usuario);
    expect(await within(painel).findByRole('figure', { name: 'Pulseira ETQ-2026-018742, válida' })).toBeInTheDocument();
    expect(painel).toHaveTextContent('Senha');
    expect(painel).toHaveTextContent('A012');
    expect(painel).toHaveTextContent('Em elaboração pelo médico');

    await usuario.click(within(painel).getByRole('button', { name: 'Reimprimir pulseira' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Reimprimir a pulseira de Antônio Carlos Ribeiro?' });
    expect(dialogo).toHaveTextContent('Antônio Carlos Ribeiro, nascimento 15/06/1954');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Reimprimir e invalidar a atual' }));
    expect(within(dialogo).getByText('Escolha o motivo da reimpressão.')).toBeInTheDocument();
    expect(pedidosDe('reimpressao')).toHaveLength(0);
    expect(await violacoesDeAcessibilidade(dialogo)).toEqual([]);

    await usuario.selectOptions(within(dialogo).getByRole('combobox', { name: 'Motivo' }), 'rasgou');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Reimprimir e invalidar a atual' }));

    expect(await within(painel).findByRole('figure', { name: 'Pulseira ETQ-2026-018790, válida' })).toBeInTheDocument();
    const antiga = within(painel).getByRole('figure', { name: 'Pulseira ETQ-2026-018742, invalidada' });
    expect(antiga).toHaveTextContent('Invalidada: rasgou. Substituída por ETQ-2026-018790.');
    expect(pedidosDe('reimpressao')[0]?.chave).toMatch(UUID);
    expect(pedidosDe('reimpressao')[0]?.corpo).toEqual({ motivo: 'rasgou' });
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it.each([
    [422, 'motivo-obrigatorio', 'A reimpressão precisa de motivo. Nada foi impresso. Escolha o motivo e repita.'],
    [409, 'etiqueta-ja-invalidada', 'A etiqueta ETQ-2026-018742 já foi invalidada. Nada foi impresso. Reimprima a partir da etiqueta válida.'],
    [403, 'sem-permissao', 'Seu perfil não pode reimprimir etiqueta. Nada foi feito.'],
  ])('reimpressão com erro %i %s mostra a mensagem do servidor no diálogo', async (status, codigo, detalhe) => {
    comAtendimentoDoAntonio();
    servidor.use(http.post('*/v1/etiquetas/:codigo/reimpressao', () => problema(status, codigo, detalhe)));
    const usuario = userEvent.setup();
    renderizar(URL);
    const painel = await abrirAntonio(usuario);
    await usuario.click(await within(painel).findByRole('button', { name: 'Reimprimir pulseira' }));
    const dialogo = await screen.findByRole('alertdialog');
    await usuario.selectOptions(within(dialogo).getByRole('combobox', { name: 'Motivo' }), 'paciente perdeu');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Reimprimir e invalidar a atual' }));
    await esperarAlertaDoServidor(detalhe, codigo, dialogo);
  });

  it('Esc fecha o diálogo de reimpressão sem enviar nada', async () => {
    comAtendimentoDoAntonio();
    const usuario = userEvent.setup();
    renderizar(URL);
    const painel = await abrirAntonio(usuario);
    await usuario.click(await within(painel).findByRole('button', { name: 'Reimprimir pulseira' }));
    await screen.findByRole('alertdialog');
    await usuario.keyboard('{Escape}');
    await waitFor(() => {
      expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    });
    expect(pedidosDe('reimpressao')).toHaveLength(0);
  });

  it('correção de nascimento pede reautenticação, volta com a MESMA chave e pede a reimpressão', async () => {
    comAtendimentoDoAntonio();
    fase1.exigirReautenticacao.add('correcao');
    const usuario = userEvent.setup();
    const primeira = renderizar(URL);
    const painel = await abrirAntonio(usuario);
    await usuario.click(await within(painel).findByRole('button', { name: 'Corrigir nascimento' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Corrigir o nascimento de Antônio Carlos Ribeiro?' });
    await digitarData(usuario, 'Nascimento correto', '15061955', dialogo);
    await usuario.type(within(dialogo).getByRole('textbox', { name: 'Motivo' }), 'documento mostra 1955');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Corrigir cadastro' }));

    const aviso = await within(dialogo).findByText(/Confirme sua senha e o segundo fator para corrigir o cadastro\./);
    expect(aviso.closest('[role="status"]')).toHaveTextContent('Por segurança, o nascimento e o motivo digitados não fica guardado fora desta tela');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Confirmar identidade' }));
    expect(primeira.irPara).toHaveBeenCalledTimes(1);
    const voltar = voltarDa(String(primeira.irPara.mock.calls[0]?.[0]));
    const chave = pedidosDe('correcao')[0]?.chave ?? '';
    expect(chave).toMatch(UUID);
    expect(voltar).toContain(`chave=${chave}`);
    expect(voltar).toContain('retomar=corrigir');
    expect(voltar).toContain(`agendamento=${ANTONIO.id}`);
    expect(voltar).not.toContain('1955');
    expect(voltar).not.toContain('documento');
    primeira.unmount();

    // Volta do provedor: a tela reabre o ato com a mesma chave.
    fase1.exigirReautenticacao.delete('correcao');
    renderizar(voltar);
    const reaberto = await screen.findByRole('alertdialog', { name: 'Corrigir o nascimento de Antônio Carlos Ribeiro?' });
    expect(reaberto).toHaveTextContent('Identidade confirmada.');
    await digitarData(usuario, 'Nascimento correto', '15061955', reaberto);
    await usuario.type(within(reaberto).getByRole('textbox', { name: 'Motivo' }), 'documento mostra 1955');
    await usuario.click(within(reaberto).getByRole('button', { name: 'Corrigir cadastro' }));

    const corrigido = await screen.findByText(/Cadastro corrigido\./);
    expect(corrigido.closest('[role="status"]')).toHaveTextContent('nascimento 15/06/1955. As etiquetas ETQ-2026-018742 foram invalidadas.');
    expect(pedidosDe('correcao')[1]?.chave).toBe(chave);
    expect(pedidosDe('correcao')[1]?.corpo).toEqual({ nascimento: '1955-06-15', motivo: 'documento mostra 1955' });
    expect(within(corrigido.closest('[role="status"]') as HTMLElement).getByRole('button', { name: 'Reimprimir pulseira' })).toBeInTheDocument();
  });
});
