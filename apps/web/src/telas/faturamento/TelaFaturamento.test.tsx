import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { esperarAlertaDoServidor, UUID } from '../../../testes/ajudantes';
import { violacoesDeAcessibilidade } from '../../../testes/axe';
import { SESSOES } from '../../../testes/dados/fase1';
import { pedidosDe } from '../../../testes/nucleo-fase1';
import { estado, reiniciarNucleo } from '../../../testes/nucleo-simulado';
import { renderizar } from '../../../testes/renderizar';

beforeEach(() => {
  reiniciarNucleo();
  estado.sessao = SESSOES.faturista;
});

const MARIA = /^Maria Helena Duarte, nascimento 12\/03\/1968/;

describe('Faturamento TISS (Faturamento.dc.html)', () => {
  it('lista as guias com os dois identificadores e mostra as pendências que o servidor achou', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/faturamento');
    await usuario.click(await screen.findByRole('button', { name: MARIA }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe' });
    const pendencias = await within(detalhe).findByRole('region', { name: 'Pendências da guia' });
    expect(within(pendencias).getByText('Laudo não assinado')).toBeInTheDocument();
    expect(within(pendencias).getByText('Bloqueia')).toBeInTheDocument();
    expect(within(pendencias).getByText('Dados do médico solicitante incompletos')).toBeInTheDocument();
    expect(within(detalhe).getByText('Conferida com AUD-UNIMED-2026.1.')).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('corrige o solicitante com motivo: a versão anterior fica guardada e a tela mostra o que o servidor devolveu', async () => {
    const usuario = userEvent.setup();
    renderizar('/faturamento');
    await usuario.click(await screen.findByRole('button', { name: MARIA }));
    const form = await screen.findByRole('form', { name: 'Corrigir a guia' });
    await usuario.click(within(form).getByRole('button', { name: 'Salvar correção' }));
    expect(within(form).getByText('Escreva o motivo (pelo menos 5 letras).')).toBeInTheDocument();
    expect(pedidosDe('correcao-conta')).toHaveLength(0);

    await usuario.type(within(form).getByRole('textbox', { name: 'Número no conselho' }), '52123456');
    await usuario.type(within(form).getByRole('textbox', { name: 'UF (código IBGE)' }), 'RJ');
    await usuario.type(within(form).getByRole('textbox', { name: 'CBO' }), '225125');
    await usuario.type(within(form).getByRole('textbox', { name: 'Motivo da correção' }), 'CRM do pedido médico');
    await usuario.click(within(form).getByRole('button', { name: 'Salvar correção' }));
    await esperarAlertaDoServidor('Os dados do solicitante não estão no padrão TISS', 'solicitante-invalido', form);

    await usuario.clear(within(form).getByRole('textbox', { name: 'UF (código IBGE)' }));
    await usuario.type(within(form).getByRole('textbox', { name: 'UF (código IBGE)' }), '33');
    await usuario.click(within(form).getByRole('button', { name: 'Salvar correção' }));
    expect(await screen.findByText('A versão anterior ficou guardada com o motivo.')).toBeInTheDocument();
    const pedidos = pedidosDe('correcao-conta');
    expect(pedidos[1]?.chave).toMatch(UUID);
    expect(pedidos[1]?.chave).not.toBe(pedidos[0]?.chave);
    expect(pedidos[1]?.corpo).toEqual({
      motivo: 'CRM do pedido médico',
      solicitante: { conselho: '06', numero: '52123456', uf: '33', cbos: '225125' },
      cid: null,
      assinatura_beneficiario: null,
      reaplicar_tabela: false,
    });
    expect(screen.getByText('Versão 2: CRM do pedido médico')).toBeInTheDocument();
  });

  it('gera o lote da operadora e mostra o envio pendente sem webservice configurado', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/faturamento?aba=lotes');
    await usuario.click(await screen.findByRole('button', { name: 'Gerar lote Unimed' }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe' });
    expect(await within(detalhe).findByRole('heading', { name: 'Lote 149, Unimed' })).toBeInTheDocument();
    expect(within(detalhe).getByText(/1 guia\(s\), R\$ 248,60\. TISS 4\.03\.00/)).toBeInTheDocument();
    expect(pedidosDe('lote')[0]?.chave).toMatch(UUID);
    await usuario.click(within(detalhe).getByRole('button', { name: 'Enviar à operadora' }));
    expect(await within(detalhe).findByText('Envio pendente: a operadora ainda não recebeu este lote.')).toBeInTheDocument();
    expect(within(detalhe).getByText(/webservice TISS da operadora não configurado\. Nada se perdeu\./)).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);
  });

  it('registra o recurso da glosa com justificativa dentro do limite do TISS', async () => {
    const usuario = userEvent.setup();
    renderizar('/faturamento?aba=glosas');
    await usuario.click(await screen.findByRole('button', { name: /^Sônia Maria Castro, nascimento 03\/02\/1962, glosa 1801/ }));
    const form = await screen.findByRole('form', { name: 'Recurso da glosa' });
    await usuario.type(within(form).getByRole('textbox', { name: 'Justificativa' }), 'curta');
    await usuario.click(within(form).getByRole('button', { name: 'Registrar recurso' }));
    expect(within(form).getByText('Escreva a justificativa (de 10 a 500 caracteres).')).toBeInTheDocument();
    expect(pedidosDe('recurso')).toHaveLength(0);
    await usuario.type(within(form).getByRole('textbox', { name: 'Justificativa' }), ': relatório clínico anexado ao pedido.');
    await usuario.click(within(form).getByRole('button', { name: 'Registrar recurso' }));
    expect(await screen.findByText('Recurso RGL-2026-000001 registrado.')).toBeInTheDocument();
    expect(pedidosDe('recurso')[0]?.chave).toMatch(UUID);
  });

  it('gerência acompanha sem botões de faturista; outros perfis veem a recusa do servidor', async () => {
    estado.sessao = SESSOES.gerente;
    const usuario = userEvent.setup();
    renderizar('/faturamento');
    await usuario.click(await screen.findByRole('button', { name: MARIA }));
    expect(await screen.findByRole('region', { name: 'Pendências da guia' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Conferir guias' })).not.toBeInTheDocument();
    expect(screen.queryByRole('form', { name: 'Corrigir a guia' })).not.toBeInTheDocument();

    estado.sessao = SESSOES.recepcao;
    renderizar('/faturamento');
    expect(await screen.findAllByText('Você não vê o faturamento')).not.toHaveLength(0);
  });
});

describe('Auditoria antes do faturamento (Auditoria-Faturamento.dc.html)', () => {
  it('mostra o resumo, as pendências por regra com o que fazer e as regras de cada operadora com a versão', async () => {
    const usuario = userEvent.setup();
    const { container } = renderizar('/faturamento/auditoria');
    const resumo = await screen.findByRole('region', { name: 'Resumo da conferência' });
    expect(within(resumo).getByText('Bloqueadas').nextSibling).toHaveTextContent('1');
    expect(within(resumo).getByText('Prontas para enviar').nextSibling).toHaveTextContent('1');
    await usuario.click(screen.getByRole('button', { name: /^Laudo não assinado/ }));
    const detalhe = screen.getByRole('region', { name: 'Detalhe da auditoria' });
    expect(within(detalhe).getByText('Avise o médico responsável pelo laudo.')).toBeInTheDocument();
    expect(within(detalhe).getByText('Maria Helena Duarte,', { exact: false })).toBeInTheDocument();
    expect(within(detalhe).getByRole('link', { name: 'Abrir a guia de Maria Helena Duarte no faturamento' })).toBeInTheDocument();
    expect(await violacoesDeAcessibilidade(container)).toEqual([]);

    await usuario.click(screen.getByRole('tab', { name: 'Regras por operadora' }));
    await usuario.click(screen.getByRole('button', { name: /^Unimed/ }));
    expect(within(detalhe).getByText('Versão AUD-UNIMED-2026.1, 5 regra(s). Usada por: Unimed.')).toBeInTheDocument();
  });

  it('conferir guias mostra o resultado do servidor', async () => {
    const usuario = userEvent.setup();
    renderizar('/faturamento/auditoria');
    await usuario.click(await screen.findByRole('button', { name: 'Conferir guias' }));
    expect(await screen.findByText('2 guia(s) conferida(s): 1 pronta(s), 1 com pendência.')).toBeInTheDocument();
    expect(pedidosDe('conferencia')[0]?.chave).toMatch(UUID);
  });
});
