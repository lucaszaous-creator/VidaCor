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

/** Orçamento simulado (regras reais em `crates/api/tests/orcamento.rs`). */
export const manipuladoresOrcamento = [
  http.get('*/v1/orcamentos/catalogo', () =>
    HttpResponse.json({
      tabela_versao: 12,
      limite_desconto_pontos_base: 1000,
      exames: [
        { tuss: '40901300', nome: 'Ecodoppler de carótidas', valor_centavos: 30000 },
        { tuss: '40901114', nome: 'Ecocardiograma transtorácico', valor_centavos: 30000 },
        { tuss: '40901122', nome: 'USG de tireoide', valor_centavos: 18000 },
      ],
      pacotes: [{ codigo: 'PAC-CARDIO', nome: 'Pacote Cardio', regra_versao: 3, tuss: ['40901114', '40901300'], valor_centavos: 54000 }],
    }),
  ),
  http.get('*/v1/orcamentos', () => HttpResponse.json([])),
  http.post('*/v1/orcamentos', async ({ request }) => {
    const c = (await registrar('emitir-orcamento', request)) as { itens: string[]; pacote?: string | null; desconto?: { pontos_base: number } | null };
    if ((c.desconto?.pontos_base ?? 0) > 1000) return problema(403, 'desconto-acima-do-limite', 'Este desconto passa do limite de 10% do orçamento. Nenhum orçamento foi emitido.');
    const pacote = c.pacote === 'PAC-CARDIO' ? 6000 : 0;
    const subtotal = c.itens.length * 30000;
    const desconto = Math.floor(((subtotal - pacote) * (c.desconto?.pontos_base ?? 0)) / 10000);
    return HttpResponse.json(
      {
        id: 'o1', codigo: 'ORC-2026-000001', paciente_id: 'p', paciente_nome: 'Vera Lúcia Antunes', paciente_nascimento: '1957-09-18', itens: [], tabela_versao: 12,
        pacote: c.pacote ?? null, pacote_regra_versao: c.pacote === 'PAC-CARDIO' ? 3 : null, subtotal_centavos: subtotal, pacote_centavos: pacote, desconto_centavos: desconto,
        desconto_pontos_base: c.desconto?.pontos_base ?? null, desconto_motivo: null, total_centavos: subtotal - pacote - desconto, pagamento_previsto: 'pix',
        valido_ate: '2026-11-20', situacao: 'emitido', motivo_cancelamento: null, criado_em: '2026-11-06T12:00:00Z',
      },
      { status: 201 },
    );
  }),
];

/** Aparelhos simulados (regras reais em `crates/api/tests/aparelhos.rs`). */
const aparelhoSimulado = (id: string, nome: string, sala: string, ae: string, extra: Partial<{ ativo: boolean; situacao: string; erro: number }> = {}) => ({
  id, codigo: `APA-${id}`, nome, ae_title: ae, modalidade: 'US', sala_id: `s-${id}`, sala_nome: sala, ativo: extra.ativo ?? true, versao: 1,
  situacao: extra.situacao ?? 'conectado',
  hoje: { listas_enviadas: 12, listas_pendentes: extra.erro ?? 0, listas_com_erro: extra.erro ?? 0, ultimo_envio: '2026-11-06T11:30:00Z', ultimo_erro: (extra.erro ?? 0) > 0 ? 'conexão recusada pelo aparelho' : null },
});
export const manipuladoresAparelhos = [
  http.get('*/v1/aparelhos', () =>
    temPerfil('ti', 'gerente')
      ? HttpResponse.json({ estudos_recebidos_hoje: 41, estudos_sem_paciente: 1, aparelhos: [aparelhoSimulado('1', 'Vivid E95', 'Sala 1', 'SALA1'), aparelhoSimulado('6', 'Revolution CT', 'Sala 6', 'SALA6', { situacao: 'atencao', erro: 2 })] })
      : semPermissao('ver os aparelhos'),
  ),
  http.post('*/v1/aparelhos/:id/mudancas', async ({ request, params }) => {
    const c = (await registrar('mudar-aparelho', request)) as { tipo: string };
    return HttpResponse.json({ ...aparelhoSimulado(String(params.id), 'Vivid E95', 'Sala 1', 'SALA1', { ativo: c.tipo === 'reativar', situacao: c.tipo === 'reativar' ? 'conectado' : 'desativado' }), versao: 2 });
  }),
  http.post('*/v1/aparelhos', async ({ request }) => {
    const c = (await registrar('cadastrar-aparelho', request)) as { nome: string; ae_title: string };
    if (c.ae_title === 'SALA1') return problema(409, 'ae-title-em-uso', 'Este AE Title já é de outro aparelho. Nada foi cadastrado.');
    return HttpResponse.json(aparelhoSimulado('9', c.nome, 'Sala 2', c.ae_title), { status: 201 });
  }),
];

/** Lista de espera simulada (regras reais em `crates/api/tests/lista_espera.rs`). */
const pedidoSimulado = (id: string, nome: string, nasc: string, prioridade: string, oferta: boolean) => ({
  id, codigo: `ESP-${id}`, posicao: Number(id), paciente: { id: `p${id}`, nome, nascimento: nasc }, exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' },
  prioridade, preferencia: 'qualquer', justificativa: prioridade === 'urgente' ? 'suspeita de valvopatia' : null, medico_solicitante: prioridade === 'urgente' ? 'Dr. Paulo Medeiros' : null,
  operadora: null, guia_codigo: null, situacao: oferta ? 'ofertado' : 'aguardando', desde: '2026-11-02T12:00:00Z',
  oferta: oferta ? { id: 'of1', codigo: 'OFE-1', sala_nome: 'Sala 1', inicio: '2026-11-09T13:00:00Z', expira_em: '2026-11-09T12:20:00Z' } : null,
});
export const manipuladoresEspera = [
  http.get('*/v1/lista-espera', () =>
    temPerfil('recepcao', 'gerente')
      ? HttpResponse.json([pedidoSimulado('1', 'Maria Helena Duarte', '1968-03-12', 'urgente', true), pedidoSimulado('2', 'Vera Lúcia Antunes', '1957-09-18', 'rotina', false)])
      : semPermissao('ver a lista de espera'),
  ),
  http.post('*/v1/lista-espera', async ({ request }) => {
    const c = (await registrar('incluir-espera', request)) as { prioridade: string };
    return HttpResponse.json(pedidoSimulado('3', 'Vera Lúcia Antunes', '1957-09-18', c.prioridade, false), { status: 201 });
  }),
  http.post('*/v1/ofertas/:id/aceite', async ({ request }) => {
    const c = (await registrar('aceitar-oferta', request)) as { confirmacao: { nome: string; nascimento: string } };
    if (c.confirmacao.nome !== 'Maria Helena Duarte' || c.confirmacao.nascimento !== '1968-03-12')
      return problema(422, 'identificacao-nao-confere', 'Nome ou data de nascimento não conferem com o cadastro do paciente da oferta. Nada foi agendado. Confira com o paciente.');
    return HttpResponse.json({ id: 'of1', codigo: 'OFE-1', espera_id: '1', espera_codigo: 'ESP-1', paciente: { id: 'p1', nome: 'Maria Helena Duarte', nascimento: '1968-03-12' }, exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' }, sala: { id: 's1', nome: 'Sala 1' }, inicio: '2026-11-09T13:00:00Z', fim: '2026-11-09T13:30:00Z', expira_em: '2026-11-09T12:20:00Z', situacao: 'aceita', origem_vaga: 'cancelamento', agendamento_codigo: 'AGD-2026-000099', motivo: null });
  }),
  http.post('*/v1/bloqueios', async ({ request }) => {
    const c = (await registrar('bloquear', request)) as { motivo: string; inicio: string; fim: string };
    return HttpResponse.json([{ id: 'b1', codigo: 'BLQ-1', sala: { id: 's2', nome: 'Sala 2' }, tipo: 'manutencao', inicio: c.inicio, fim: c.fim, motivo: c.motivo, situacao: 'ativo', retirado_em: null, motivo_retirada: null }], { status: 201 });
  }),
];

/** Abertura de imagens simulada (regras reais em `crates/api/tests/visualizacao.rs`). */
export const manipuladoresVisualizacao = [
  http.get('*/v1/estudos/:id/visualizacao', () =>
    temPerfil('medico', 'tecnico')
      ? HttpResponse.json({ estudo_codigo: 'EST-2026-000081', study_uid: '1.2.826.0.1.3680043.8.498.2', imagens: 42, url: 'http://127.0.0.1:3000/viewer?StudyInstanceUIDs=1.2.826.0.1.3680043.8.498.2' })
      : semPermissao('abrir as imagens'),
  ),
];

/** Integrações simuladas (regras reais em `crates/api/tests/configuracoes.rs`). */
export const integracoesSimuladas = [
  { id: 'email', nome: 'E-mail (SMTP)', para_que: 'Convite e acesso ao portal do paciente.', situacao: 'em_uso', o_que_falta: null, pendentes_no_outbox: 0, variaveis: [{ nome: 'VIDACOR_SMTP_URL', descricao: 'Servidor SMTP.', obrigatoria: true, segredo: true, preenchida: true }] },
  { id: 'whatsapp', nome: 'WhatsApp (Cloud API da Meta)', para_que: 'Confirmação de horário e avisos.', situacao: 'incompleta', o_que_falta: 'Falta preencher: VIDACOR_WHATSAPP_NUMERO_ID.', pendentes_no_outbox: 3, variaveis: [{ nome: 'VIDACOR_WHATSAPP_NUMERO_ID', descricao: 'ID do número.', obrigatoria: true, segredo: false, preenchida: false }, { nome: 'VIDACOR_WHATSAPP_TOKEN', descricao: 'Token.', obrigatoria: true, segredo: true, preenchida: true }] },
  { id: 'nfse', nome: 'NFS-e (prefeitura)', para_que: 'Nota fiscal do particular.', situacao: 'nao_configurada', o_que_falta: 'Preencha no ambiente do servidor: VIDACOR_NFSE_PROVEDOR.', pendentes_no_outbox: 0, variaveis: [{ nome: 'VIDACOR_NFSE_PROVEDOR', descricao: 'Provedor.', obrigatoria: true, segredo: false, preenchida: false }] },
];
export const manipuladoresConfiguracoes = [
  http.get('*/v1/configuracoes/integracoes', () => (temPerfil('ti', 'gerente') ? HttpResponse.json(integracoesSimuladas) : semPermissao('ver as configurações'))),
];
