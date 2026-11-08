/**
 * Núcleo simulado do faturamento TISS. Imita só o que a tela precisa mostrar; as regras de verdade
 * (auditoria por operadora, lote atômico, XSD, glosa e recurso) são provadas no domínio
 * (`crates/dominio-faturamento/tests`) e nos testes G4 em `crates/api/tests/faturamento.rs`.
 */
import type { ContaDetalhe, Glosa, LoteDetalhe, PedidoCorrecaoConta, PedidoLote, PedidoRecurso } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { CONTRATO_UNIMED, faturamentoInicial, painelDe, type EstadoDoFaturamento } from './dados/faturamento';
import { uuid } from './dados/ficticios';
import { registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { problema } from './problema';

export const faturamento: { e: EstadoDoFaturamento } = { e: faturamentoInicial() };

export function reiniciarFaturamento(): void {
  faturamento.e = faturamentoInicial();
}

const ler = () => temPerfil('faturista', 'gerente');
const escrever = () => temPerfil('faturista');

function contaPorId(id: unknown): ContaDetalhe | undefined {
  return faturamento.e.contas.find((c) => c.resumo.id === id);
}

function loteDetalhe(id: unknown): LoteDetalhe | undefined {
  const lote = faturamento.e.lotes.find((l) => l.id === id);
  if (lote === undefined) return undefined;
  return { lote, contas: faturamento.e.contas.filter((c) => c.resumo.lote_codigo === lote.codigo).map((c) => c.resumo) };
}

export const manipuladoresFaturamento = [
  http.get('*/v1/faturamento/contas', () => (ler() ? HttpResponse.json(faturamento.e.contas.map((c) => c.resumo)) : semPermissao('ver o faturamento'))),
  http.get('*/v1/faturamento/contas/:id', ({ params }) => {
    if (!ler()) return semPermissao('ver o faturamento');
    const c = contaPorId(params.id);
    return c === undefined ? problema(404, 'conta-nao-encontrada', 'Esta guia não existe no faturamento desta unidade. Nada foi alterado.') : HttpResponse.json(c);
  }),
  http.post('*/v1/faturamento/contas/:id/correcao', async ({ request, params }) => {
    const corpo = (await registrar('correcao-conta', request)) as PedidoCorrecaoConta;
    if (!escrever()) return semPermissao('corrigir a guia');
    const c = contaPorId(params.id);
    if (c === undefined) return problema(404, 'conta-nao-encontrada', 'Esta guia não existe no faturamento desta unidade. Nada foi alterado.');
    if (corpo.solicitante?.uf !== undefined && !/^\d{2}$/.test(corpo.solicitante.uf)) {
      return problema(422, 'solicitante-invalido', 'Os dados do solicitante não estão no padrão TISS: conselho com 2 dígitos (06 = CRM), número do conselho, UF com 2 dígitos (33 = RJ) e CBO com 6 dígitos. Nada foi gravado.');
    }
    c.historico = [{ versao: c.versao, motivo: corpo.motivo, criado_em: '2026-11-08T13:00:00Z' }, ...c.historico];
    c.versao += 2;
    if (corpo.solicitante != null) {
      c.solicitante = { nome: c.solicitante.nome, ...corpo.solicitante };
      c.pendencias = c.pendencias.filter((p) => p.regra !== 'solicitante-incompleto');
    }
    c.resumo.pendencias_que_impedem = c.pendencias.filter((p) => p.efeito !== 'atencao').length;
    c.resumo.situacao = c.resumo.pendencias_que_impedem === 0 ? 'pronta' : 'com_pendencia';
    return HttpResponse.json(c);
  }),
  http.post('*/v1/faturamento/conferencias', async ({ request }) => {
    await registrar('conferencia', request);
    if (!escrever()) return semPermissao('conferir as guias');
    const abertas = faturamento.e.contas.filter((c) => ['pronta', 'com_pendencia'].includes(c.resumo.situacao));
    const prontas = abertas.filter((c) => c.resumo.situacao === 'pronta').length;
    return HttpResponse.json({ novas: 0, conferidas: abertas.length, prontas, com_pendencia: abertas.length - prontas });
  }),
  http.get('*/v1/faturamento/auditoria', () => (ler() ? HttpResponse.json(painelDe(faturamento.e)) : semPermissao('ver a auditoria'))),
  http.get('*/v1/faturamento/contratos', () => {
    if (!ler()) return semPermissao('ver os contratos');
    const prontas = faturamento.e.contas.filter((c) => c.resumo.situacao === 'pronta').length;
    return HttpResponse.json([{ ...CONTRATO_UNIMED, contas_prontas: prontas }]);
  }),
  http.get('*/v1/faturamento/lotes', () => (ler() ? HttpResponse.json(faturamento.e.lotes) : semPermissao('ver os lotes'))),
  http.get('*/v1/faturamento/lotes/:id', ({ params }) => {
    if (!ler()) return semPermissao('ver o lote');
    const d = loteDetalhe(params.id);
    return d === undefined ? problema(404, 'lote-nao-encontrado', 'Este lote não existe no faturamento desta unidade.') : HttpResponse.json(d);
  }),
  http.post('*/v1/faturamento/lotes', async ({ request }) => {
    const corpo = (await registrar('lote', request)) as PedidoLote;
    if (!escrever()) return semPermissao('gerar o lote');
    if (corpo.contrato_id !== CONTRATO_UNIMED.id) return problema(404, 'contrato-nao-encontrado', 'Esta operadora não tem contrato ativo nesta unidade. Nada foi gerado.');
    const prontas = faturamento.e.contas.filter((c) => c.resumo.situacao === 'pronta');
    if (prontas.length === 0) {
      return problema(409, 'nenhuma-guia-pronta', 'Nenhuma guia desta operadora está pronta para o lote. Nada foi gerado. Confira as pendências na auditoria.');
    }
    const n = faturamento.e.lotes.length + 1;
    const codigo = `LTI-2026-${String(n).padStart(6, '0')}`;
    for (const c of prontas) {
      c.resumo.situacao = 'em_lote';
      c.resumo.lote_codigo = codigo;
    }
    faturamento.e.lotes.unshift({
      id: uuid('000000f3', n),
      codigo,
      numero: 148 + n - 1,
      contrato_id: CONTRATO_UNIMED.id,
      operadora: 'Unimed',
      quantidade_guias: prontas.length,
      valor_total_centavos: prontas.reduce((s, c) => s + c.resumo.valor_total_centavos, 0),
      situacao: 'gerado',
      versao_tiss: '4.03.00',
      hash_tiss: '9c1d2e3f405162738495a6b7c8d9e0f1',
      sha256: 'b'.repeat(64),
      protocolo: null,
      criado_em: '2026-11-08T13:10:00Z',
      enviado_em: null,
      envio_pendente: null,
      valor_pago_centavos: null,
      valor_glosado_centavos: null,
    });
    return HttpResponse.json(loteDetalhe(uuid('000000f3', n)), { status: 201 });
  }),
  http.post('*/v1/faturamento/lotes/:id/envio', async ({ request, params }) => {
    await registrar('envio-lote', request);
    if (!escrever()) return semPermissao('enviar o lote');
    const lote = faturamento.e.lotes.find((l) => l.id === params.id);
    if (lote === undefined) return problema(404, 'lote-nao-encontrado', 'Este lote não existe no faturamento desta unidade.');
    if (lote.situacao === 'gerado') {
      lote.situacao = 'aguardando_envio';
      lote.envio_pendente = { tentativas: 1, ultimo_erro: 'webservice TISS da operadora não configurado' };
    }
    return HttpResponse.json(loteDetalhe(lote.id), { status: 202 });
  }),
  http.get('*/v1/faturamento/glosas', () => (ler() ? HttpResponse.json(faturamento.e.glosas) : semPermissao('ver as glosas'))),
  http.post('*/v1/faturamento/glosas/:id/recurso', async ({ request, params }) => {
    const corpo = (await registrar('recurso', request)) as PedidoRecurso;
    if (!escrever()) return semPermissao('recorrer da glosa');
    const g: Glosa | undefined = faturamento.e.glosas.find((x) => x.id === params.id);
    if (g === undefined) return problema(404, 'glosa-nao-encontrada', 'Esta glosa não existe no faturamento desta unidade.');
    if (g.situacao !== 'aberta') return problema(409, 'recurso-ja-registrado', 'Esta glosa já tem recurso registrado. Nada foi alterado.');
    g.situacao = 'em_recurso';
    g.recurso = { id: uuid('000000f5', 1), codigo: 'RGL-2026-000001', justificativa: corpo.justificativa.trim(), situacao: 'aguardando_envio', criado_em: '2026-11-08T13:20:00Z' };
    return HttpResponse.json(g, { status: 201 });
  }),
];
