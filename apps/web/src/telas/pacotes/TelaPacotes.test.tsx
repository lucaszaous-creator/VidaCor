import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { digitarData, esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Check-up e pacotes (Checkup-Pacotes.dc.html)', () => {
  it('a recepção vê o pacote vigente com a ordem, o preparo e o avulso calculado pelo servidor, sem mudar nada', async () => {
    estado.sessao = SESSOES.recepcao;
    const { container } = renderizar('/pacotes');
    const detalhe = await screen.findByRole('region', { name: 'Detalhe do pacote' });
    expect(await within(detalhe).findByRole('heading', { name: 'Check-up cardiológico' })).toBeInTheDocument();
    expect(within(detalhe).getByText('R$ 690,00')).toBeInTheDocument();
    expect(within(detalhe).getByText('Avulso R$ 890,00, 22,47% a menos')).toBeInTheDocument();
    expect(within(detalhe).getByText('Tênis e roupa leve; refeição leve até 2 h antes')).toBeInTheDocument();
    expect(within(detalhe).getByText('Por último: o esforço muda frequência e pressão.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Novo pacote' })).not.toBeInTheDocument();
    expect(within(detalhe).queryByRole('button', { name: 'Nova versão' })).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('pacote que ficou mais caro que o avulso aparece com o aviso de que o orçamento não o aplica', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacotes');
    await usuario.click(await screen.findByRole('button', { name: /Check-up da mulher/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do pacote' });
    expect(within(detalhe).getByText('O orçamento não aplica este pacote.')).toBeInTheDocument();
  });

  it('a gerência publica versão nova com a versão vista; o servidor recusa preço que não sai mais barato', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacotes');
    const detalhe = await screen.findByRole('region', { name: 'Detalhe do pacote' });
    await usuario.click(await within(detalhe).findByRole('button', { name: 'Nova versão' }));
    const form = within(detalhe).getByRole('form', { name: 'Nova versão, Check-up cardiológico' });
    const preco = within(form).getByRole('textbox', { name: 'Preço do pacote (R$)' });
    expect(preco).toHaveValue('720,00');
    await digitarData(usuario, 'Começa em', '01022027', form);
    await usuario.clear(preco);
    await usuario.type(preco, '900,00');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo' }), 'reajuste do pacote');
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão' }));
    await esperarAlertaDoServidor('O preço do pacote precisa ser menor que a soma avulsa', 'pacote-nao-sai-mais-barato', form);
    await usuario.clear(preco);
    await usuario.type(preco, '700,00');
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão' }));
    expect(await screen.findByRole('heading', { name: 'Check-up cardiológico' })).toBeInTheDocument();
    const pedidos = pedidosDe('nova-versao-pacote');
    expect(pedidos.at(-1)?.corpo).toMatchObject({ regra_versao_vista: 4, vigente_desde: '2027-02-01', preco_centavos: 70_000, motivo: 'reajuste do pacote' });
    expect((pedidos.at(-1)?.corpo as { exames: { tuss: string }[] }).exames.map((e) => e.tuss)).toEqual(['40901300', '40901114', '40901220']);
    expect(pedidos.at(-1)?.chave).not.toBe(pedidos[0]?.chave);
  });

  it('o formulário de pacote novo pede de 2 a 10 exames antes de enviar', async () => {
    const usuario = userEvent.setup();
    renderizar('/pacotes');
    await usuario.click(await screen.findByRole('button', { name: 'Novo pacote' }));
    const form = screen.getByRole('form', { name: 'Novo pacote' });
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão' }));
    expect(within(form).getByText('O pacote tem de 2 a 10 exames.')).toBeInTheDocument();
    expect(pedidosDe('criar-pacote')).toHaveLength(0);
  });

  it('o caixa não consulta pacotes: a tela diz e não mostra preço', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/pacotes');
    expect(await screen.findByText('Você não consulta os pacotes')).toBeInTheDocument();
    expect(screen.queryByText('R$ 690,00')).not.toBeInTheDocument();
  });
});
