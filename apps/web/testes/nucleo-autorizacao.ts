/**
 * Núcleo simulado da autorização pedida pelo sistema. Imita só o que a tela mostra; as regras de
 * verdade (canal pelo contrato, um pedido por agendamento, resposta gravada com hash, guia preenchida
 * e reserva confirmada) são provadas em `crates/api/tests/autorizacao.rs` e no domínio.
 */
import type { AutorizacaoDetalhe, NovoPedidoAutorizacao, PedidoProtocolo, RespostaDoPortal } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { autorizacoesIniciais } from './dados/autorizacao';
import { uuid } from './dados/ficticios';
import { registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { problema } from './problema';

export const autorizacoes: { lista: AutorizacaoDetalhe[] } = { lista: autorizacoesIniciais() };

export function reiniciarAutorizacoes(): void {
  autorizacoes.lista = autorizacoesIniciais();
}

const ler = () => temPerfil('recepcao', 'faturista', 'gerente');
const pedir = () => temPerfil('recepcao', 'faturista');
const achar = (id: unknown) => autorizacoes.lista.find((a) => a.resumo.id === id);

export const manipuladoresAutorizacao = [
  http.get('*/v1/autorizacoes', () => (ler() ? HttpResponse.json(autorizacoes.lista.map((a) => a.resumo)) : semPermissao('ver as autorizações'))),
  http.get('*/v1/autorizacoes/:id', ({ params }) => {
    if (!ler()) return semPermissao('ver a autorização');
    const a = achar(params.id);
    return a === undefined ? problema(404, 'autorizacao-nao-encontrada', 'Este pedido de autorização não existe nesta unidade. Nada foi alterado.') : HttpResponse.json(a);
  }),
  http.post('*/v1/autorizacoes', async ({ request }) => {
    const c = (await registrar('autorizacao', request)) as NovoPedidoAutorizacao;
    if (!pedir()) return semPermissao('pedir autorização');
    if (!/^\d{2}$/.test(c.solicitante.uf)) {
      return problema(422, 'solicitante-invalido', 'Os dados do médico solicitante não estão no padrão TISS: nome, conselho com 2 dígitos (06 = CRM), número, UF com 2 dígitos (33 = RJ) e CBO com 6 dígitos. Nada foi pedido.');
    }
    const n = autorizacoes.lista.length + 1;
    const webservice = c.operadora.trim().toLowerCase() === 'unimed';
    const base = autorizacoes.lista[1] as AutorizacaoDetalhe;
    const nova: AutorizacaoDetalhe = {
      ...base,
      resumo: {
        ...base.resumo,
        id: uuid('000000a7', n),
        codigo: `AUT-2026-${String(n).padStart(6, '0')}`,
        paciente: { nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' },
        operadora: c.operadora.trim(),
        canal: webservice ? 'webservice' : 'portal',
        situacao: webservice ? 'aguardando_envio' : 'aguardando_portal',
        agendamento_id: c.agendamento_id,
      },
    };
    autorizacoes.lista.unshift(nova);
    return HttpResponse.json(nova, { status: 201 });
  }),
  http.post('*/v1/autorizacoes/:id/protocolo', async ({ request, params }) => {
    const c = (await registrar('protocolo', request)) as PedidoProtocolo;
    if (!pedir()) return semPermissao('registrar o protocolo');
    const a = achar(params.id);
    if (a === undefined) return problema(404, 'autorizacao-nao-encontrada', 'Este pedido de autorização não existe nesta unidade.');
    if (!/^[A-Za-z0-9./-]{1,40}$/.test(c.protocolo)) {
      return problema(422, 'protocolo-invalido', 'Copie o protocolo como aparece no portal da operadora: letras, números, ponto, barra e hífen, até 40. Nada foi gravado.');
    }
    a.resumo = { ...a.resumo, situacao: 'enviada', protocolo: c.protocolo };
    a.enviada_em = '2026-11-08T21:00:00Z';
    return HttpResponse.json(a);
  }),
  http.post('*/v1/autorizacoes/:id/resposta', async ({ request, params }) => {
    const c = (await registrar('resposta-portal', request)) as RespostaDoPortal;
    if (!pedir()) return semPermissao('registrar a resposta');
    const a = achar(params.id);
    if (a === undefined) return problema(404, 'autorizacao-nao-encontrada', 'Este pedido de autorização não existe nesta unidade.');
    if (c.situacao === 'autorizada' && (c.senha == null || c.validade == null)) {
      return problema(422, 'resposta-invalida', 'Autorizada pede a senha (até 20 letras e números) e a validade, como no portal. Nada foi gravado.');
    }
    a.resumo = { ...a.resumo, situacao: c.situacao };
    a.respostas = [{ origem: 'portal', sha256: 'abc'.padEnd(64, '1'), situacao_lida: c.situacao, erro_leitura: null, hash_confere: true, aplicada: true, recebida_em: '2026-11-08T21:10:00Z' }];
    if (c.situacao === 'autorizada') {
      a.guia = { id: uuid('000000a9', 9), codigo: 'GUI-2026-000099', numero: c.numero_guia_operadora ?? a.resumo.codigo, senha: c.senha ?? '', validade: c.validade ?? '', situacao: 'consumida' };
      a.situacao_do_agendamento = 'confirmado';
    }
    return HttpResponse.json(a);
  }),
  http.post('*/v1/autorizacoes/:id/consulta', async ({ request, params }) => {
    await registrar('consulta', request);
    if (!pedir()) return semPermissao('consultar a operadora');
    const a = achar(params.id);
    if (a === undefined) return problema(404, 'autorizacao-nao-encontrada', 'Este pedido de autorização não existe nesta unidade.');
    a.pendente_no_outbox = { tentativas: 0, proxima_em: '2026-11-08T21:00:00Z', ultimo_erro: null };
    return HttpResponse.json(a, { status: 202 });
  }),
];
