import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
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

  it('a gerência publica o cabeçalho dos documentos; a TI não vê a opção', async () => {
    const usuario = userEvent.setup();
    const ti = renderizar('/configuracoes');
    const nav = await screen.findByRole('navigation', { name: 'Outras configurações' });
    expect(within(nav).queryByRole('button', { name: 'Cabeçalho dos documentos' })).not.toBeInTheDocument();
    ti.unmount();
    estado.sessao = SESSOES.gerente;
    const { container } = renderizar('/configuracoes');
    await usuario.click(await screen.findByRole('button', { name: 'Cabeçalho dos documentos' }));
    const painel = screen.getByRole('region', { name: 'Cabeçalho dos documentos' });
    expect(await within(painel).findByText('Cabeçalho não cadastrado.')).toBeInTheDocument();
    const form = within(painel).getByRole('form', { name: 'Cabeçalho dos documentos' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Nome fantasia' }), 'VidaCor Imagem Centro');
    await usuario.type(within(form).getByRole('textbox', { name: 'CNPJ' }), '11.222.333/0001-82');
    await usuario.type(within(form).getByRole('textbox', { name: 'Razão social' }), 'VidaCor Diagnóstico por Imagem Ltda.');
    await usuario.type(within(form).getByRole('textbox', { name: 'Endereço' }), 'Rua Fictícia, 100, Centro');
    await usuario.type(within(form).getByRole('textbox', { name: 'Telefone' }), '(11) 3000-0000');
    await usuario.type(within(form).getByRole('textbox', { name: 'Responsável técnico' }), 'Dra. Cecília Souto');
    await usuario.selectOptions(within(form).getByRole('combobox', { name: 'UF do CRM' }), 'SP');
    await usuario.type(within(form).getByRole('textbox', { name: 'CRM' }), '123456');
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão nova' }));
    await esperarAlertaDoServidor('O CNPJ não confere', 'cabecalho-invalido');
    await usuario.clear(within(form).getByRole('textbox', { name: 'CNPJ' }));
    await usuario.type(within(form).getByRole('textbox', { name: 'CNPJ' }), '11.222.333/0001-81');
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
    await usuario.click(within(form).getByRole('button', { name: 'Publicar versão nova' }));
    expect(await within(painel).findByText('Cabeçalho versão 1 publicado.')).toBeInTheDocument();
    expect(await within(painel).findByText('Dra. Cecília Souto, CRM-SP 123456')).toBeInTheDocument();
    expect(pedidosDe('publicar-cabecalho')[1]?.corpo).toMatchObject({ numero_atual: null, cnpj: '11.222.333/0001-81', responsavel_crm_uf: 'SP', responsavel_crm_numero: '123456', site: null, motivo: null });
  });

  it('leva às outras configurações já existentes', async () => {
    renderizar('/configuracoes');
    const nav = await screen.findByRole('navigation', { name: 'Outras configurações' });
    expect(within(nav).getByRole('link', { name: 'Usuários e perfis' })).toHaveAttribute('href', '/usuarios');
    expect(within(nav).getByRole('link', { name: 'Aparelhos' })).toHaveAttribute('href', '/aparelhos');
  });
});
