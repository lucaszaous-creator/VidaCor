import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.ti;
});

describe('Configurações (Configuracoes.dc.html)', () => {
  it('mostra o que falta em cada integração sem nenhum valor e os envios parados', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/configuracoes');
    await usuario.click(await screen.findByRole('button', { name: /WhatsApp \(Cloud API da Meta\)/ }));
    const d = screen.getByRole('region', { name: 'Detalhe da integração' });
    expect(within(d).getByText('Falta preencher: VIDACOR_WHATSAPP_NUMERO_ID.')).toBeInTheDocument();
    expect(within(d).getByText('3 envio(s) aguardando esta integração.')).toBeInTheDocument();
    expect(within(d).getByText('VIDACOR_WHATSAPP_TOKEN')).toBeInTheDocument();
    expect(within(d).getAllByText('Preenchida')).toHaveLength(1);
    expect(within(d).getAllByText('Vazia')).toHaveLength(1);
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('leva às outras configurações já existentes', async () => {
    renderizar('/configuracoes');
    const nav = await screen.findByRole('navigation', { name: 'Outras configurações' });
    expect(within(nav).getByRole('link', { name: 'Usuários e perfis' })).toHaveAttribute('href', '/usuarios');
    expect(within(nav).getByRole('link', { name: 'Aparelhos' })).toHaveAttribute('href', '/aparelhos');
  });
});
