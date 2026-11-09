/**
 * Núcleo simulado da minha agenda do médico, gestão de dose e relatórios para os testes de TELA. As
 * regras reais estão em `crates/api/tests/minha_agenda.rs`, `gestao_dose.rs` e `relatorios.rs`, com
 * PostgreSQL e Orthanc reais. Só dados fictícios.
 */
import type { AusenciaDoMedico, DecisaoDaAusencia, NovaAusencia, NovaJustificativa } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { reautenticacao, registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';
import { SHA_FICTICIO } from './nucleo-documentos';
import { agenda, ausenciaAguardando, CATALOGO, doses, HOJE } from './dados/gestao';

export const gestaoSimulada = {
  ausencias: [ausenciaAguardando()],
  doses: doses(),
  exportacoes: 0,
};

export function reiniciarGestao(): void {
  gestaoSimulada.ausencias = [ausenciaAguardando()];
  gestaoSimulada.doses = doses();
  gestaoSimulada.exportacoes = 0;
}

export const manipuladoresGestao = [
  http.get('*/v1/minha-agenda', () => (temPerfil('medico') ? HttpResponse.json(agenda()) : semPermissao('abrir a agenda do médico'))),
  http.post('*/v1/minha-agenda/ausencias', async ({ request }) => {
    const n = (await registrar('ausencia', request)) as NovaAusencia;
    if (!temPerfil('medico')) return semPermissao('pedir bloqueio de agenda');
    const nova: AusenciaDoMedico = { ...ausenciaAguardando(), id: '00000011-0000-7000-8000-000000000002', codigo: 'AUS-2026-000043', inicio: n.inicio, fim: n.fim, motivo: n.motivo, detalhe: n.detalhe ?? null, afetados_no_pedido: 3 };
    gestaoSimulada.ausencias.push(nova);
    return HttpResponse.json(nova, { status: 201 });
  }),
  http.get('*/v1/ausencias', () => {
    if (temPerfil('gerente', 'recepcao')) return HttpResponse.json(gestaoSimulada.ausencias);
    if (temPerfil('medico')) return HttpResponse.json(gestaoSimulada.ausencias.filter((a) => a.medico_id === estado.sessao.usuario.id));
    return semPermissao('ver os pedidos de bloqueio');
  }),
  http.post('*/v1/ausencias/:id/decisao', async ({ request, params }) => {
    const d = (await registrar('decisao-ausencia', request)) as DecisaoDaAusencia;
    if (!temPerfil('gerente')) return semPermissao('decidir pedido de bloqueio');
    const a = gestaoSimulada.ausencias.find((x) => x.id === params.id);
    if (a === undefined) return problema(404, 'ausencia-nao-encontrada', 'Este pedido de bloqueio não existe ou não é seu.');
    if (!d.aprovar && (d.motivo ?? '').trim().length < 5) return problema(422, 'motivo-obrigatorio', 'Para recusar, diga o motivo ao médico (5 a 300 caracteres). Nada foi alterado.');
    Object.assign(a, { situacao: d.aprovar ? 'aprovada' : 'recusada', decidido_por: estado.sessao.usuario.nome, decidido_em: '2026-11-06T12:00:00Z', motivo_decisao: d.motivo ?? null, afetados_na_decisao: d.aprovar ? 14 : null });
    return HttpResponse.json(a);
  }),
  http.get('*/v1/doses', () => (temPerfil('medico') ? HttpResponse.json(gestaoSimulada.doses) : semPermissao('ver a dose por paciente'))),
  http.get('*/v1/doses/protocolos', () =>
    temPerfil('medico', 'gerente')
      ? HttpResponse.json([{ exame_tuss: '41001020', exame: 'TC de tórax', aparelho: 'Somatom go.Up', grandeza: 'ctdivol', exames: 21, mediana: 10.6, nrd: 12, percentual_do_nrd: 88, acima: 1 }])
      : semPermissao('ver a dose por protocolo'),
  ),
  http.get('*/v1/doses/pacientes/:id', () =>
    HttpResponse.json({
      paciente: { id: '0000000e-0000-7000-8000-000000000002', nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' },
      exames: [{ exame: 'TC de tórax', aparelho: 'Somatom go.Up', realizado_em: '2026-11-07T12:12:00Z', dlp_mgycm: 520, dose_efetiva_msv: 7.28 }],
      dose_efetiva_12_meses_msv: 7.28,
      exames_sem_estimativa: 0,
    }),
  ),
  http.post('*/v1/doses/:id/justificativa', async ({ request, params }) => {
    const n = (await registrar('justificativa', request)) as NovaJustificativa;
    if (!temPerfil('medico')) return semPermissao('justificar dose');
    const e = gestaoSimulada.doses.exames.find((x) => x.dose_id === params.id);
    if (e === undefined) return problema(404, 'dose-nao-encontrada', 'Exame com dose não encontrado ou sem vínculo de atendimento com você. Nada foi registrado.');
    e.justificativa = { motivo: n.motivo, detalhe: n.detalhe ?? null, por: estado.sessao.usuario.nome, em: '2026-11-07T12:24:00Z' };
    gestaoSimulada.doses.sem_justificativa = 0;
    return HttpResponse.json(e.justificativa, { status: 201 });
  }),
  http.get('*/v1/nrd', () =>
    temPerfil('medico', 'gerente')
      ? HttpResponse.json([
          { tuss: '41001020', exame: 'TC de tórax', modalidade: 'CT', numero: 1, grandeza: 'ctdivol', valor: 12, fator_dose_efetiva: 0.014, motivo: null, publicado_em: '2026-03-01T12:00:00Z' },
          { tuss: '40801012', exame: 'Raio X de tórax', modalidade: 'DX', numero: null, grandeza: null, valor: null },
        ])
      : semPermissao('ver o NRD'),
  ),
  http.get('*/v1/relatorios', () => {
    const meus = CATALOGO.filter((r) => r.perfis.some((p) => estado.sessao.usuario.perfis.includes(p))).map((r) => ({ id: r.id, grupo: r.grupo, titulo: r.titulo, descricao: r.descricao, dado_de_saude: r.dado_de_saude }));
    return meus.length === 0 ? semPermissao('ver relatórios') : HttpResponse.json(meus);
  }),
  http.get('*/v1/relatorios/exportacoes', () =>
    HttpResponse.json(
      gestaoSimulada.exportacoes === 0
        ? []
        : [{ codigo: 'EXP-2026-000001', relatorio: 'faltas', titulo: 'Faltas', formato: 'csv', de: '2026-11-01', ate: '2026-11-06', conteudo: 'pseudonimizado', linhas: 1, sha256: SHA_FICTICIO, quem: estado.sessao.usuario.nome, quando: '2026-11-06T13:40:00Z' }],
    ),
  ),
  http.get('*/v1/relatorios/:id', ({ params, request }) => {
    const r = CATALOGO.find((x) => x.id === params.id);
    if (r === undefined) return problema(404, 'relatorio-inexistente', 'Este relatório não existe no catálogo.');
    if (!r.perfis.some((p) => estado.sessao.usuario.perfis.includes(p))) return semPermissao('ver este relatório');
    const url = new URL(request.url);
    const faltas = r.id === 'faltas';
    return HttpResponse.json({
      id: r.id,
      titulo: r.titulo,
      de: url.searchParams.get('de') ?? '2026-11-01',
      ate: url.searchParams.get('ate') ?? HOJE,
      colunas: faltas ? ['Paciente', 'Exame', 'Faltas'] : ['Médico', 'Principal exame', 'Laudos'],
      linhas: faltas ? [{ a: 'PAC-2026-000193', b: 'TUSS 40901173', valor: 2, c: '2' }] : [{ a: 'Dr. Paulo Medeiros', b: 'Ecocardiograma', valor: 412, c: '412' }],
      dado_de_saude: r.dado_de_saude,
      gerado_em: '2026-11-06T13:40:00Z',
    });
  }),
  http.post('*/v1/relatorios/:id/exportacao', async ({ request, params }) => {
    await registrar('exportacao', request);
    const re = reautenticacao('exportar-relatorio');
    if (re !== null) return re;
    gestaoSimulada.exportacoes += 1;
    return new HttpResponse('﻿Paciente;Exame;Faltas\r\nPAC-2026-000193;TUSS 40901173;2\r\n', {
      status: 200,
      headers: {
        'Content-Type': 'text/csv; charset=utf-8',
        'Content-Disposition': `attachment; filename="EXP-2026-000001-${String(params.id)}-2026-11-01-2026-11-06.csv"`,
        'X-Documento-Sha256': SHA_FICTICIO,
        'X-Documento-Codigo': 'EXP-2026-000001',
      },
    });
  }),
];
