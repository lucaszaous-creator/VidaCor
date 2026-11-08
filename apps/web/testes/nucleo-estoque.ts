/** Núcleo simulado do estoque (regras reais em `crates/api/tests/estoque.rs`). */
import type { ItemDeEstoque } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { SONIA } from './dados/fase1';
import { registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { problema } from './problema';

export const CONTRASTE: ItemDeEstoque = {
  id: '00000028-0000-7000-8000-000000000001',
  codigo: 'ITM-CTR',
  nome: 'Contraste iodado não iônico 350, 100 ml',
  medida: 'frasco',
  minimo: 20,
  local: 'Tomografia',
  contraste: true,
  saldo_valido: 8,
  abaixo_do_minimo: true,
  lotes: [
    { id: '00000029-0000-7000-8000-000000000001', numero: 'L2408-77', validade: '2027-03-31', saldo: 3, vencido: false },
    { id: '00000029-0000-7000-8000-000000000002', numero: 'L2501-12', validade: '2027-09-30', saldo: 5, vencido: false },
  ],
};

export const manipuladoresEstoque = [
  http.get('*/v1/estoque/itens', () => (temPerfil('estoque', 'gerente', 'tecnico', 'enfermagem') ? HttpResponse.json([CONTRASTE]) : semPermissao('ver o estoque'))),
  http.post('*/v1/estoque/entradas', async ({ request }) => {
    const c = (await registrar('entrada-estoque', request)) as { numero_lote: string; validade: string; quantidade: number };
    if (!temPerfil('estoque', 'gerente')) return semPermissao('dar entrada no estoque');
    return HttpResponse.json({ movimento_id: 'm1', lote_id: 'l1', numero_lote: c.numero_lote.toUpperCase(), validade: c.validade, saldo_do_lote: c.quantidade }, { status: 201 });
  }),
  http.get('*/v1/estoque/lotes/:id/rastreio', () =>
    temPerfil('gerente')
      ? HttpResponse.json({ numero: 'L2408-77', item: CONTRASTE.nome, validade: '2027-03-31', pacientes: [{ paciente: SONIA.paciente, atendimento_codigo: 'ATD-2026-000031', quantidade: 1, aplicado_em: '2026-11-06T12:10:00Z' }] })
      : semPermissao('rastrear lote'),
  ),
  http.post('*/v1/atendimentos/:id/contraste', async ({ request }) => {
    await registrar('contraste', request);
    if (!temPerfil('tecnico', 'enfermagem')) return semPermissao('registrar contraste');
    return HttpResponse.json({ movimento_id: 'm2', lote_id: CONTRASTE.lotes[0]?.id, numero_lote: 'L2408-77', validade: '2027-03-31', saldo_do_lote: 2 }, { status: 201 });
  }),
];

export const semSaldo = () => problema(409, 'sem-lote-valido', 'Não há lote dentro da validade com saldo para este contraste. Nada foi baixado. Avise o estoque.');

/** Lastro simulado (regras reais em `crates/api/tests/lastro_explorador.rs`). */
export const lastroSimulado = { integra: true };
export const manipuladoresLastro = [
  http.get('*/v1/lastro/cadeia', ({ request }) => {
    if (!temPerfil('gerente', 'auditor')) return semPermissao('abrir o lastro');
    const codigo = new URL(request.url).searchParams.get('codigo') ?? '';
    if (!codigo.startsWith('ATD') && !codigo.startsWith('ACC')) return problema(404, 'codigo-sem-cadeia', 'Nenhum atendimento desta unidade tem este código.');
    return HttpResponse.json({
      paciente: SONIA.paciente,
      atendimento_codigo: 'ATD-2026-000031',
      nos: [
        { tipo: 'agendamento', id: 'n1', codigo: 'AG-2611-4100', situacao: 'em_atendimento', criado_em: '2026-11-05T12:00:00Z' },
        { tipo: 'atendimento', id: 'n2', codigo: 'ATD-2026-000031', situacao: 'aguardando_sala', criado_em: '2026-11-06T11:00:00Z' },
        { tipo: 'etiqueta', id: 'n3', codigo: 'ETQ-2026-000090', situacao: 'valida', criado_em: '2026-11-06T11:00:00Z' },
      ],
      eventos: [
        { sequencia: 10, dominio: 'recepcao', tipo: 'checkin', registro_codigo: 'ATD-2026-000031', criado_em: '2026-11-06T11:00:00Z', origem: 'recepcao', hash: '3f9ac21b77e0', integro: true },
        { sequencia: 11, dominio: 'recepcao', tipo: 'etiqueta_emitida', registro_codigo: 'ETQ-2026-000090', criado_em: '2026-11-06T11:00:01Z', origem: 'recepcao', hash: '7b02e5d9a1c4', integro: lastroSimulado.integra },
      ],
      integra: lastroSimulado.integra,
    });
  }),
];

/** Indicadores simulados (regras reais em `crates/api/tests/indicadores.rs`). */
export const manipuladoresIndicadores = [
  http.get('*/v1/indicadores', ({ request }) => {
    if (!temPerfil('gerente', 'financeiro')) return semPermissao('ver indicadores');
    const u = new URL(request.url);
    return HttpResponse.json({
      de: u.searchParams.get('de'),
      ate: u.searchParams.get('ate'),
      agendamentos: 120,
      atendimentos: 104,
      faltas: 9,
      taxa_de_faltas: 0.08,
      receita_centavos: 4480000,
      recebimentos: 118,
      ticket_medio_centavos: 37966,
      tempo_medio_de_laudo_horas: 6.4,
      producao: [
        { exame: 'Ecocardiograma transtorácico', atendimentos: 41 },
        { exame: 'USG de tireoide', atendimentos: 22 },
      ],
      por_medico: [
        { medico: 'Dr. Paulo Medeiros', laudos: 60, horas_medias: 5.2 },
        { medico: 'Dr. Marcos Freire', laudos: 20, horas_medias: 11.5 },
      ],
      ocupacao: [
        { dia_da_semana: 1, hora: 8, agendamentos: 12 },
        { dia_da_semana: 2, hora: 9, agendamentos: 4 },
      ],
    });
  }),
];
