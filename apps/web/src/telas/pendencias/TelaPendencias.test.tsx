import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, voltarDa } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES, SONIA } from '../../../testes/dados/fase1';
import { fase1, pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

const URL = '/caixa/pendencias';
const NOME_NA_LISTA = /^Sônia Maria Castro, nascimento 03\/02\/1962/;

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Pendências do caixa (gerência e financeiro)', () => {
  it('lista os descontos pedidos e autoriza confirmando nome e nascimento', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar(URL);
    expect(await screen.findByRole('tab', { name: 'Descontos (1)' })).toBeInTheDocument();
    await usuario.click(await screen.findByRole('button', { name: NOME_NA_LISTA }));
    expect(screen.getByRole('heading', { name: 'Desconto de 15,00% pedido por Carla Mendes' })).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(screen.getByRole('button', { name: 'Autorizar 15,00%' }));
    const dialogo = await screen.findByRole('dialog', { name: 'Autorizar 15,00% de desconto para Sônia Maria Castro?' });
    expect(dialogo).toHaveTextContent('03/02/1962');
    await usuario.click(within(dialogo).getByRole('button', { name: 'Autorizar desconto' }));
    expect(await screen.findByText(/Desconto de 15,00% autorizado para Sônia Maria Castro\./)).toBeInTheDocument();
    expect(pedidosDe('autorizacao')[0]?.corpo).toEqual({
      paciente_id: SONIA.paciente.id,
      pontos_base: 1500,
      motivo: 'Tabela social',
      solicitante_id: SESSOES.caixa.usuario.id,
      pedido_id: '00000025-0000-7000-8000-000000000001',
    });
  });

  it('janela bloqueada: autorizar sai para reautenticar e volta repetindo com a mesma chave', async () => {
    fase1.exigirReautenticacao.add('autorizacao');
    const usuario = userEvent.setup();
    const primeira = renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: NOME_NA_LISTA }));
    await usuario.click(screen.getByRole('button', { name: 'Autorizar 15,00%' }));
    await usuario.click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Autorizar desconto' }));
    await usuario.click(await screen.findByRole('button', { name: 'Confirmar identidade' }));
    const voltar = voltarDa(String(primeira.irPara.mock.calls[0]?.[0]));
    expect(voltar).not.toContain('Sônia');
    primeira.unmount();
    fase1.exigirReautenticacao.delete('autorizacao');
    renderizar(voltar);
    const dialogo = await screen.findByRole('dialog', { name: 'Autorizar 15,00% de desconto para Sônia Maria Castro?' });
    await usuario.click(within(dialogo).getByRole('button', { name: 'Autorizar desconto' }));
    expect(await screen.findByText(/autorizado para Sônia Maria Castro/)).toBeInTheDocument();
    const [a, b] = pedidosDe('autorizacao');
    expect(b?.chave).toBe(a?.chave);
  });

  it('recusa pede motivo e o envia ao servidor', async () => {
    const usuario = userEvent.setup();
    renderizar(URL);
    await usuario.click(await screen.findByRole('button', { name: NOME_NA_LISTA }));
    await usuario.click(screen.getByRole('button', { name: 'Recusar' }));
    expect(screen.getByText('Escreva o motivo, com pelo menos 5 letras.')).toBeInTheDocument();
    expect(pedidosDe('recusar-desconto')).toHaveLength(0);
    await usuario.type(screen.getByRole('textbox', { name: 'Motivo da recusa (o caixa lê)' }), 'fora da política social');
    await usuario.click(screen.getByRole('button', { name: 'Recusar' }));
    expect(await screen.findByText(/Desconto recusado para Sônia Maria Castro\./)).toBeInTheDocument();
    expect(pedidosDe('recusar-desconto')[0]?.corpo).toEqual({ motivo: 'fora da política social' });
  });

  it('estorno: aprovação irreversível mostra paciente e valor', async () => {
    const usuario = userEvent.setup();
    renderizar(`${URL}?aba=estornos`);
    await usuario.click(await screen.findByRole('button', { name: NOME_NA_LISTA }));
    expect(screen.getByRole('heading', { name: 'Estorno de R$ 380,00 pedido por Carla Mendes' })).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: 'Aprovar estorno' }));
    const dialogo = await screen.findByRole('alertdialog', { name: 'Estornar R$ 380,00 para Sônia Maria Castro?' });
    await usuario.click(within(dialogo).getByRole('button', { name: 'Aprovar estorno' }));
    expect(await screen.findByText(/Estorno de R\$ 380,00 aprovado para Sônia Maria Castro\./)).toBeInTheDocument();
  });

  it('quem pediu o estorno não decide: a tela mostra a recusa do servidor', async () => {
    estado.sessao = SESSOES.caixa;
    const usuario = userEvent.setup();
    renderizar(`${URL}?aba=estornos`);
    await usuario.click(await screen.findByRole('button', { name: NOME_NA_LISTA }));
    await usuario.click(screen.getByRole('button', { name: 'Aprovar estorno' }));
    await usuario.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Aprovar estorno' }));
    await esperarAlertaDoServidor('Quem pediu o estorno não pode aprovar o próprio pedido.', 'separacao-de-funcoes');
  });

  it('endereço antigo de aprovação de estorno leva às pendências', async () => {
    renderizar('/caixa/estornos?estorno=00000022-0000-7000-8000-000000000001');
    expect(await screen.findByRole('heading', { name: 'Estorno de R$ 380,00 pedido por Carla Mendes' })).toBeInTheDocument();
  });

  it('recepção não vê pendências do caixa', async () => {
    estado.sessao = SESSOES.recepcao;
    renderizar(URL);
    expect(await screen.findByText('Você não decide descontos')).toBeInTheDocument();
  });
});
