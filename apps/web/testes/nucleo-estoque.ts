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

/** Comunicação simulada (regras reais em `crates/api/tests/comunicacao.rs`). */
export const comunicacaoSimulada = { whatsapp: true };
export const manipuladoresComunicacao = [
  http.get('*/v1/pacientes/:id/comunicacao', () =>
    temPerfil('recepcao', 'gerente')
      ? HttpResponse.json({ whatsapp: comunicacaoSimulada.whatsapp, motivo: comunicacaoSimulada.whatsapp ? null : 'pedido no balcão', mensagens: [{ modelo: 'confirmacao', situacao: 'enviada', resposta: 'confirmou', criado_em: '2026-11-05T12:00:00Z' }] })
      : semPermissao('ver os avisos do paciente'),
  ),
  http.post('*/v1/pacientes/:id/comunicacao', async ({ request }) => {
    const c = (await registrar('preferencia-whatsapp', request)) as { whatsapp: boolean; motivo: string };
    comunicacaoSimulada.whatsapp = c.whatsapp;
    return HttpResponse.json({ whatsapp: c.whatsapp, motivo: c.motivo, mensagens: [] });
  }),
  http.post('*/v1/comunicacao/lembretes', async ({ request }) => {
    await registrar('lembretes', request);
    return HttpResponse.json({ enfileirados: 7 });
  }),
];

/** Financeiro simulado (regras reais em `crates/api/tests/financeiro.rs`). */
const tituloBase = { tipo: 'pagar', descricao: 'Contraste iodado, NF 1203', contraparte: 'Distribuidora Fictícia', categoria: 'fornecedor', valor_centavos: 1500000, vencimento: '2026-11-20', situacao: 'aberto', aprovacoes_exigidas: 2, aprovacoes: 1, vencido: false };
export const manipuladoresFinanceiro = [
  http.get('*/v1/financeiro/resumo', () =>
    temPerfil('financeiro', 'gerente')
      ? HttpResponse.json({ entradas_do_mes_centavos: 41270000, saidas_do_mes_centavos: 28730000, saldo_em_contas_centavos: 53890000, a_pagar_aberto_centavos: 1500000, a_receber_aberto_centavos: 31280000, aguardando_aprovacao: 1, extrato_sem_conciliacao: 3 })
      : semPermissao('acessar o financeiro'),
  ),
  http.get('*/v1/financeiro/contas', () => HttpResponse.json([{ id: 'c1', codigo: 'CTA-1', nome: 'Conta movimento' }])),
  http.get('*/v1/financeiro/titulos', ({ request }) => {
    const tipo = new URL(request.url).searchParams.get('tipo');
    return HttpResponse.json(tipo === 'pagar' ? [{ ...tituloBase, id: 't1', codigo: 'TIT-2026-000010' }] : []);
  }),
  http.post('*/v1/financeiro/titulos', async ({ request }) => {
    const c = (await registrar('lancar-titulo', request)) as { valor_centavos: number; tipo: string };
    return HttpResponse.json({ ...tituloBase, ...c, id: 't2', codigo: 'TIT-2026-000011', aprovacoes: 0, aprovacoes_exigidas: c.tipo === 'pagar' ? (c.valor_centavos > 1000000 ? 2 : 1) : 0 }, { status: 201 });
  }),
  http.post('*/v1/financeiro/titulos/:id/aprovacoes', async ({ request }) => {
    await registrar('aprovar-titulo', request);
    return HttpResponse.json({ ...tituloBase, id: 't1', codigo: 'TIT-2026-000010', aprovacoes: 2 });
  }),
  http.get('*/v1/financeiro/repasse', () => HttpResponse.json([{ medico_id: 'm1', medico: 'Dr. Paulo Medeiros', laudos: 60, base_centavos: 2400000, percentual_pb: 3000, regra_versao: 2, valor_centavos: 720000, sem_preco: 0 }, { medico_id: 'm2', medico: 'Dr. Marcos Freire', laudos: 4, base_centavos: 160000, percentual_pb: null, regra_versao: null, valor_centavos: null, sem_preco: 0 }])),
  http.post('*/v1/financeiro/repasse/:comp/titulos', async ({ request }) => {
    await registrar('gerar-repasse', request);
    return HttpResponse.json({ titulos_criados: 1, ja_existiam: 0, sem_regra: ['Dr. Marcos Freire'] });
  }),
];

/** Usuários simulados (regras reais em `crates/api/tests/usuarios.rs`). */
const renata = { id: 'u1', codigo: 'USR-renata.alves', login: 'renata.alves', nome: 'Dra. Renata Alves', perfis: ['medico'], ativo: false, ultimo_acesso: null, pendente: { id: 'p1', tipo: 'criar', perfil: null, motivo: 'contratação, ultrassom', pedido_por: 'Marcos Teixeira' } };
export const usuariosCriados: unknown[] = [];
export const manipuladoresUsuarios = [
  http.get('*/v1/usuarios', () => (temPerfil('ti', 'gerente') ? HttpResponse.json([renata, { ...renata, id: 'u2', codigo: 'USR-carla.caixa', login: 'carla.caixa', nome: 'Carla Mendes', perfis: ['caixa'], ativo: true, pendente: null }, ...usuariosCriados]) : semPermissao('administrar usuários'))),
  http.post('*/v1/usuarios', async ({ request }) => {
    const c = (await registrar('criar-usuario', request)) as { login: string; nome: string; perfis: string[] };
    const criado = { ...renata, id: 'u3', ...c, ativo: !c.perfis.includes('medico') };
    usuariosCriados.push(criado);
    return HttpResponse.json(criado, { status: 201 });
  }),
  http.post('*/v1/usuarios/:id/mudancas', async ({ request }) => {
    const c = (await registrar('mudar-perfil', request)) as { tipo: string; perfil?: string };
    if (c.tipo === 'conceder' && c.perfil === 'financeiro') return problema(422, 'separacao-de-funcoes', 'Quem recebe no caixa não concilia no financeiro. Nada foi alterado.');
    return HttpResponse.json({ ...renata, id: 'u2', nome: 'Carla Mendes', perfis: ['caixa'], ativo: c.tipo !== 'desativar', pendente: null });
  }),
  http.post('*/v1/usuarios/pedidos/:id/decisao', async ({ request }) => {
    await registrar('decidir-perfil', request);
    return HttpResponse.json({ ...renata, ativo: true, pendente: null });
  }),
];

/** NFS-e simulada: emitida para qualquer recebimento pago. */
export const manipuladoresNfse = [
  http.get('*/v1/recebimentos/:id/nota', () =>
    HttpResponse.json({ numero: '004812', codigo_verificacao: 'A1B2C3D4', valor_centavos: 38000, situacao: 'emitida', emitida_em: '2026-11-06T12:00:00Z', cancelada_em: null }),
  ),
];

/** Exame externo simulado (regras reais em `crates/api/tests/exame_externo.rs`). */
export const manipuladoresExterno = [
  http.get('*/v1/pacientes/:id/exames-externos', () => HttpResponse.json([])),
  http.post('*/v1/pacientes/:id/exames-externos', async ({ request }) => {
    const c = (await registrar('importar-externo', request)) as { nome_na_origem: string; nascimento_na_origem: string; estudos: unknown[] };
    if (c.nascimento_na_origem !== '1957-09-18') return problema(409, 'nascimento-diferente', 'O nascimento no exame externo é diferente do cadastro. Nada foi importado: pode ser exame de outra pessoa.');
    return HttpResponse.json({ id: 'x1', codigo: 'EXT-2026-000001', instituicao: 'Hospital São Lucas', origem: 'link', nome_na_origem: c.nome_na_origem, id_na_origem: null, criado_em: '2026-11-06T12:00:00Z', estudos: c.estudos, somente_leitura: true }, { status: 201 });
  }),
];

/** TV de chamada simulada (regras reais em `crates/api/tests/painel_de_chamada.rs`). */
export const chamadasSimuladas: { senha: string; nome_abreviado: string; destino: string; rechamada: boolean; chamada_em: string }[] = [];
export const manipuladoresPainel = [
  http.get('*/v1/painel-de-chamada', () => {
    if (!temPerfil('tv', 'recepcao', 'gerente')) return semPermissao('ver o painel de chamada');
    const [agora, ...ultimas] = [...chamadasSimuladas].reverse();
    return HttpResponse.json({ agora: agora ?? null, ultimas: ultimas.slice(0, 4) });
  }),
  http.post('*/v1/atendimentos/:id/chamadas', async ({ request }) => {
    const c = (await registrar('chamar', request)) as { destino: string; guiche?: number };
    if (!temPerfil('recepcao', 'tecnico')) return semPermissao('chamar a senha');
    const nova = {
      senha: 'A001',
      nome_abreviado: 'Antônio C. R.',
      destino: c.destino === 'guiche' ? `Guichê ${String(c.guiche)}` : 'Sala 1',
      rechamada: chamadasSimuladas.length > 0,
      chamada_em: '2026-11-06T11:40:00Z',
    };
    chamadasSimuladas.push(nova);
    return HttpResponse.json(nova, { status: 201 });
  }),
];
