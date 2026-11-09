/**
 * Etiquetas e rastreio simulados para os testes de TELA. As regras reais (estado pelo lastro, resolução e
 * invalidação atômicas, acesso indevido) estão em `crates/api/tests/etiquetas_rastreio.rs`.
 */
import type { DetalheDaEtiqueta, LinhaDeEtiqueta, PainelDeEtiquetas } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { semPermissao, temPerfil } from './nucleo-fase1';
import { problema } from './problema';

const JULIANA = { id: '0190a0b0-0000-7000-8000-000000000003', nome: 'Juliana Sampaio Rocha', nascimento: '1993-04-12' };
const MARIA = { id: '0190a0b0-0000-7000-8000-000000000002', nome: 'Maria Helena Duarte', nascimento: '1968-03-12' };

function inicial(): { etiquetas: LinhaDeEtiqueta[]; divergenciaAberta: boolean; pedidos: { rota: string; corpo: unknown }[] } {
  return {
    divergenciaAberta: true,
    pedidos: [],
    etiquetas: [
      { codigo: 'ETQ-2026-000002', tipo: 'pulseira', estado: 'divergencia', paciente: JULIANA, exame: 'USG morfológica', sala: 'Sala 2', accession: 'ACC-2026-000002', ultimo_evento: { descricao: 'Bipada na sala errada (Sala 1): exame bloqueado, nada enviado ao aparelho', em: '2026-11-09T11:44:00Z' } },
      { codigo: 'ETQ-2026-000001', tipo: 'pulseira', estado: 'em_uso', paciente: MARIA, exame: 'Ecocardiograma transtorácico', sala: 'Sala 1', accession: 'ACC-2026-000001', ultimo_evento: { descricao: 'Bipada na Sala 1, paciente e exame conferem; aparelho Vivid E95', em: '2026-11-09T11:41:00Z' } },
    ],
  };
}

export const etiquetasSimuladas = inicial();

export function reiniciarEtiquetas(): void {
  Object.assign(etiquetasSimuladas, inicial());
}

const ve = (): boolean => temPerfil('recepcao', 'gerente', 'tecnico', 'medico');

export const manipuladoresEtiquetas = [
  http.get('*/v1/etiquetas', () => {
    if (!ve()) return semPermissao('ver as etiquetas');
    const s = etiquetasSimuladas;
    const painel: PainelDeEtiquetas = {
      dia: '2026-11-09',
      impressas: 86,
      bipadas: 41,
      reimpressoes: 3,
      invalidadas: 0,
      divergencias_abertas: s.divergenciaAberta ? 1 : 0,
      divergencias: s.divergenciaAberta
        ? [{ id: 'div-1', codigo: 'DIV-2026-000001', etiqueta: 'ETQ-2026-000002', paciente: JULIANA, exame: 'USG morfológica', sala_bipada: 'Sala 1', sala_correta: 'Sala 2', sala_correta_id: 'sala-2', aberta_em: '2026-11-09T11:44:00Z' }]
        : [],
      etiquetas: s.etiquetas,
    };
    return HttpResponse.json(painel);
  }),
  http.get('*/v1/etiquetas/:codigo', ({ params }) => {
    if (!ve()) return semPermissao('ver as etiquetas');
    const e = etiquetasSimuladas.etiquetas.find((x) => x.codigo === params.codigo);
    if (e === undefined) return problema(404, 'etiqueta-nao-encontrada', 'Etiqueta não encontrada nesta unidade. Confira o código bipado.');
    const d: DetalheDaEtiqueta = {
      etiqueta: e,
      eventos: [
        { tipo: 'impressa', descricao: 'Impressa no GUICHE-1', em: '2026-11-09T11:19:00Z', por: 'Carla Mendes', nivel: 'ok' },
        ...(e.codigo === 'ETQ-2026-000002' ? [{ tipo: 'bloqueada' as const, descricao: 'Bipada na sala errada (Sala 1): exame bloqueado, nada enviado ao aparelho', em: '2026-11-09T11:44:00Z', por: 'Bruno Sales', nivel: 'alerta' as const }] : []),
      ],
    };
    return HttpResponse.json(d);
  }),
  http.post('*/v1/etiquetas/:codigo/invalidacao', async ({ params, request }) => {
    if (!temPerfil('recepcao')) return semPermissao('invalidar etiquetas');
    const corpo = (await request.json()) as { motivo: string };
    etiquetasSimuladas.pedidos.push({ rota: 'invalidar', corpo });
    const e = etiquetasSimuladas.etiquetas.find((x) => x.codigo === params.codigo);
    if (e === undefined) return problema(404, 'etiqueta-nao-encontrada', 'Etiqueta não encontrada nesta unidade. Nada mudou.');
    e.estado = 'invalidada';
    e.motivo_invalidacao = corpo.motivo;
    return HttpResponse.json({ codigo: e.codigo, situacao: 'invalidada', motivo_invalidacao: corpo.motivo });
  }),
  http.post('*/v1/divergencias-de-sala/:id/resolucao', async ({ request }) => {
    if (!temPerfil('recepcao', 'tecnico', 'gerente')) return semPermissao('resolver divergências');
    etiquetasSimuladas.pedidos.push({ rota: 'resolver', corpo: await request.json() });
    if (!etiquetasSimuladas.divergenciaAberta) return problema(409, 'divergencia-indisponivel', 'Esta divergência já foi resolvida por outra pessoa. Nada mudou; veja o histórico da etiqueta.');
    etiquetasSimuladas.divergenciaAberta = false;
    return HttpResponse.json({ id: 'div-1', codigo: 'DIV-2026-000001', situacao: 'resolvida', descricao: 'paciente encaminhado à Sala 2' });
  }),
];
