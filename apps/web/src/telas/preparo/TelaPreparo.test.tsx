import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';
import { passosDoTexto } from './FormularioDoPreparo';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.gerente;
});

describe('Preparo de exames (Preparo-Exame.dc.html)', () => {
  it('a recepção lê os passos vigentes e as versões, sem publicar', async () => {
    estado.sessao = SESSOES.recepcao;
    const usuario = userEvent.setup();
    const { container } = renderizar('/preparo');
    expect(await screen.findByText('2 exames sem orientação')).toBeInTheDocument();
    await usuario.click(screen.getByRole('button', { name: /Ecocardiograma com estresse/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do preparo' });
    const passos = within(detalhe).getByRole('list', { name: 'Passos do preparo vigente' });
    expect(within(passos).getAllByRole('listitem')).toHaveLength(4);
    expect(await within(detalhe).findByText('Versão 1')).toBeInTheDocument();
    expect(within(detalhe).queryByRole('button', { name: 'Nova versão' })).not.toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('a gerência publica a versão nova com motivo e a versão vista vai ao servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/preparo');
    await usuario.click(await screen.findByRole('button', { name: /Ecocardiograma com estresse/ }));
    await usuario.click(screen.getByRole('button', { name: 'Nova versão' }));
    const form = screen.getByRole('form', { name: 'Nova versão do preparo' });
    const area = within(form).getByRole('textbox', { name: 'Passos, um por linha' });
    await usuario.clear(area);
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão' }));
    expect(within(form).getByText('Escreva pelo menos um passo.')).toBeInTheDocument();
    expect(within(form).getByText('Diga por que o preparo mudou (5 letras ou mais).')).toBeInTheDocument();
    await usuario.type(area, 'Jejum de 4 horas.{Enter}{Enter}Traga o pedido médico.');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo da mudança' }), 'Novo protocolo do aparelho');
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão' }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe do preparo' });
    expect(await within(detalhe).findByText('Versão 3')).toBeInTheDocument();
    expect(pedidosDe('preparo')[0]?.corpo).toEqual({
      versao_vista: 2,
      passos: ['Jejum de 4 horas.', 'Traga o pedido médico.'],
      chegar_antes_min: 30,
      motivo: 'Novo protocolo do aparelho',
    });
  });

  it('o caixa não consulta o preparo', async () => {
    estado.sessao = SESSOES.caixa;
    renderizar('/preparo');
    expect(await screen.findByText('Você não consulta o preparo')).toBeInTheDocument();
  });

  it('uma linha por passo, sem linhas vazias nem espaços sobrando', () => {
    expect(passosDoTexto('  Jejum   de 4 horas. \n\n Traga o pedido.\n')).toEqual(['Jejum de 4 horas.', 'Traga o pedido.']);
  });
});
