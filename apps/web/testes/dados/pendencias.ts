/** Pendências fictícias do caixa (Sônia, pedido por Carla Mendes). */
import type { EstornoDetalhado, PedidoDesconto } from '@vidacor/api-client';
import { uuid } from './ficticios.ts';
import { SESSOES, SONIA } from './fase1.ts';

export function pedidoDaSonia(): PedidoDesconto {
  return {
    id: uuid('00000025', 1),
    codigo: 'PDD-2026-000014',
    paciente: SONIA.paciente,
    pontos_base: 1500,
    motivo: 'Tabela social',
    solicitante: { id: SESSOES.caixa.usuario.id, nome: SESSOES.caixa.usuario.nome },
    situacao: 'pedido',
    autorizacao_id: null,
    decidido_por: null,
    motivo_decisao: null,
    criado_em: '2026-11-06T11:41:00Z',
  };
}

export function estornoDaSonia(): EstornoDetalhado {
  return {
    id: uuid('00000022', 1),
    codigo: 'EST-2026-000081',
    situacao: 'pedido',
    motivo: 'cobrança em duplicidade',
    recebimento_id: uuid('00000026', 1),
    recebimento_codigo: 'REC-2026-004120',
    paciente: SONIA.paciente,
    total_centavos: 38000,
    forma: 'pix',
    pago_em: '2026-11-06T11:20:00Z',
    pedido_por: { id: SESSOES.caixa.usuario.id, nome: SESSOES.caixa.usuario.nome },
    pedido_em: '2026-11-06T11:30:00Z',
    decidido_por: null,
    motivo_recusa: null,
  };
}

