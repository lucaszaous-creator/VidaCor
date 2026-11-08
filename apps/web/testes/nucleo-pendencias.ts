/**
 * Núcleo simulado das pendências do caixa (pedido de desconto e estornos). Imita as regras do
 * servidor que a tela precisa mostrar: fila, quem pediu não decide, recusa com motivo. As regras de
 * verdade são provadas nos testes G4 em `crates/api/tests/pendencias_caixa.rs`.
 */
import type { EstornoDetalhado, NovoPedidoDesconto, PedidoDesconto } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { uuid } from './dados/ficticios';
import { estornoDaSonia, pedidoDaSonia } from './dados/pendencias';
import { reautenticacao, registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';

export const AUTORIZACAO_DO_PEDIDO = uuid('00000021', 2);

export const pendencias = { descontos: new Map<string, PedidoDesconto>(), estornos: new Map<string, EstornoDetalhado>() };

export function reiniciarPendencias(): void {
  const p = pedidoDaSonia();
  const e = estornoDaSonia();
  pendencias.descontos = new Map([[p.id, p]]);
  pendencias.estornos = new Map([[e.id, e]]);
}
reiniciarPendencias();

const decisor = () => ({ id: estado.sessao.usuario.id, nome: estado.sessao.usuario.nome });

export const manipuladoresPendencias = [
  http.get('*/v1/pedidos-desconto', () => {
    if (!temPerfil('gerente', 'caixa', 'financeiro')) return semPermissao('ver pedidos de desconto');
    return HttpResponse.json([...pendencias.descontos.values()].filter((p) => p.situacao === 'pedido'));
  }),
  http.post('*/v1/pedidos-desconto', async ({ request }) => {
    const corpo = (await registrar('pedir-desconto', request)) as NovoPedidoDesconto;
    if (!temPerfil('caixa')) return semPermissao('pedir desconto');
    const p: PedidoDesconto = { ...pedidoDaSonia(), id: uuid('00000025', 9), pontos_base: corpo.pontos_base, motivo: corpo.motivo };
    pendencias.descontos.set(p.id, p);
    return HttpResponse.json(p, { status: 201 });
  }),
  http.get('*/v1/pedidos-desconto/:id', ({ params }) => {
    const p = pendencias.descontos.get(String(params.id));
    return p === undefined ? problema(404, 'pedido-nao-encontrado', 'Pedido de desconto não encontrado.') : HttpResponse.json(p);
  }),
  http.post('*/v1/pedidos-desconto/:id/recusa', async ({ request, params }) => {
    const corpo = (await registrar('recusar-desconto', request)) as { motivo: string };
    const p = pendencias.descontos.get(String(params.id));
    if (p === undefined) return problema(404, 'pedido-nao-encontrado', 'Pedido de desconto não encontrado.');
    if (!temPerfil('gerente')) return semPermissao('recusar desconto');
    Object.assign(p, { situacao: 'recusado', motivo_decisao: corpo.motivo, decidido_por: decisor() });
    return HttpResponse.json(p);
  }),
  http.post('*/v1/pedidos-desconto/:id/cancelamento', async ({ request, params }) => {
    await registrar('cancelar-desconto', request);
    const p = pendencias.descontos.get(String(params.id));
    if (p === undefined) return problema(404, 'pedido-nao-encontrado', 'Pedido de desconto não encontrado.');
    Object.assign(p, { situacao: 'cancelado', decidido_por: decisor() });
    return HttpResponse.json(p);
  }),
  // Autorização pela fila: fecha o pedido (o servidor faz isto na mesma transação).
  http.post('*/v1/autorizacoes-desconto', async ({ request }) => {
    const corpo = (await registrar('autorizacao', request)) as { pedido_id?: string; solicitante_id: string };
    if (!temPerfil('gerente')) return semPermissao('autorizar desconto');
    if (corpo.solicitante_id === estado.sessao.usuario.id) {
      return problema(403, 'separacao-de-funcoes', 'Você não pode autorizar um desconto pedido por você. Nada foi autorizado.');
    }
    const re = reautenticacao('autorizacao');
    if (re !== null) return re;
    const p = pendencias.descontos.get(corpo.pedido_id ?? '');
    if (p !== undefined) Object.assign(p, { situacao: 'autorizado', autorizacao_id: AUTORIZACAO_DO_PEDIDO, decidido_por: decisor() });
    return HttpResponse.json({ id: AUTORIZACAO_DO_PEDIDO, codigo: 'AUD-2026-000078', valida_ate: '2026-11-06T13:41:00Z' }, { status: 201 });
  }),
  http.get('*/v1/estornos', () => {
    if (!temPerfil('gerente', 'financeiro', 'caixa')) return semPermissao('ver estornos');
    return HttpResponse.json([...pendencias.estornos.values()].filter((e) => e.situacao === 'pedido'));
  }),
  http.post('*/v1/estornos/:id/recusa', async ({ request, params }) => {
    const corpo = (await registrar('recusar-estorno', request)) as { motivo: string };
    const e = pendencias.estornos.get(String(params.id));
    if (e === undefined) return problema(404, 'estorno-nao-encontrado', 'Pedido de estorno não encontrado.');
    if (e.pedido_por.id === estado.sessao.usuario.id) {
      return problema(403, 'separacao-de-funcoes', 'Você pediu este estorno e não pode decidi-lo. Nada foi alterado.');
    }
    Object.assign(e, { situacao: 'recusado', motivo_recusa: corpo.motivo, decidido_por: decisor() });
    return HttpResponse.json(e);
  }),
];
