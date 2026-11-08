/**
 * Núcleo simulado por `page.route` para a verificação "a tela cabe na janela". Só imita as respostas
 * do contrato (docs/29 e docs/30) com os dados fictícios; a regra é provada nos testes do núcleo.
 * Só sintaxe apagável de TypeScript: roda direto no Node 22.
 */
import type { Page, Route } from 'playwright';
import {
  ANTONIO,
  APARELHO_SALA_1,
  ATENDIMENTO_ANTONIO,
  ATENDIMENTO_VERA,
  caixaAberto,
  contingenciaAberta,
  filaDaSala1,
  filaDaSala6,
  filaDeLaudos,
  horariosLivres,
  itensDoPortal,
  laudoAntonio,
  PACIENTES,
  PORTAL_LAUDO_ECO,
  recebimentoSonia,
  SALA_6,
  SESSOES,
  TRIAGEM_PRECISA_APROVACAO,
} from '../dados/fase1.ts';
import { agendaDoDia, atendimentoDe, itensDaAgenda, SALAS } from '../dados/ficticios.ts';
import { autorizacoesIniciais } from '../dados/autorizacao.ts';
import { CONTRATO_UNIMED, faturamentoInicial, painelDe } from '../dados/faturamento.ts';
import { achadosDoLaudoAntonio } from '../dados/laudo-estruturado.ts';
import { estornoDaSonia, pedidoDaSonia } from '../dados/pendencias.ts';

export interface EstadoDaSimulacao {
  sessao: (typeof SESSOES)[keyof typeof SESSOES];
  checkinFeito: boolean;
  caixaAberto: boolean;
  contingenciaAberta: boolean;
  semVinculo: boolean;
}

export function novoEstado(): EstadoDaSimulacao {
  return { sessao: SESSOES.recepcao, checkinFeito: false, caixaAberto: false, contingenciaAberta: false, semVinculo: false };
}

function json(route: Route, corpo: unknown, status = 200): Promise<void> {
  return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(corpo) });
}

function problema(route: Route, status: number, codigo: string, detail: string, extras: Record<string, unknown> = {}): Promise<void> {
  return route.fulfill({
    status,
    contentType: 'application/problem+json',
    body: JSON.stringify({ type: 'about:blank', title: codigo, status, detail, codigo, trace_id: `trace-${codigo}`, ...extras }),
  });
}

export async function simularNucleo(page: Page, estado: EstadoDaSimulacao): Promise<void> {
  await page.route('**/v1/**', async (route) => {
    const url = new URL(route.request().url());
    const metodo = route.request().method();
    const c = url.pathname;
    if (c === '/v1/saude') return json(route, { situacao: 'ok' });
    if (c === '/v1/sessao') return json(route, estado.sessao);
    if (c === '/v1/salas') return json(route, SALAS);
    if (c === '/v1/eventos') return route.fulfill({ status: 200, contentType: 'text/event-stream', body: 'retry: 600000\n\n' });
    if (c === '/v1/contingencias/aberta') return json(route, estado.contingenciaAberta ? contingenciaAberta() : null);
    if (c === '/v1/agenda') {
      const agenda = agendaDoDia(url.searchParams.get('data') ?? '', url.searchParams.get('sala_id'));
      if (estado.checkinFeito) {
        for (const i of agenda.itens) {
          if (i.id === ANTONIO.id) {
            i.situacao = 'em_atendimento';
            i.atendimento_id = atendimentoDe(ANTONIO, 1).id;
          }
        }
      }
      return json(route, agenda);
    }
    if (c === '/v1/atendimentos' && metodo === 'POST') {
      estado.checkinFeito = true;
      const d = atendimentoDe(ANTONIO, 1);
      const criado = { id: d.id, codigo: d.codigo, accession: d.accession, agendamento_id: d.agendamento_id, paciente: d.paciente };
      return json(route, { ...criado, situacao: d.situacao, senha: d.senha, pulseira: d.pulseira, criado_em: d.criado_em }, 201);
    }
    if (c === `/v1/atendimentos/${ATENDIMENTO_VERA.id}`) return json(route, { ...ATENDIMENTO_VERA, triagem: TRIAGEM_PRECISA_APROVACAO });
    if (c.startsWith('/v1/atendimentos/')) {
      const id = c.split('/').at(-1);
      const item = itensDaAgenda().find((i) => i.atendimento_id === id);
      return json(route, item === undefined ? ATENDIMENTO_ANTONIO : { ...atendimentoDe(item, 1), laudo_situacao: 'rascunho' });
    }
    if (c === '/v1/pacientes') {
      const t = (url.searchParams.get('busca') ?? '').toLocaleLowerCase('pt-BR');
      return json(route, PACIENTES.filter((p) => p.nome.toLocaleLowerCase('pt-BR').includes(t)));
    }
    if (/^\/v1\/salas\/[^/]+\/fila$/.test(c)) return json(route, c.includes(SALA_6.id) ? filaDaSala6(TRIAGEM_PRECISA_APROVACAO) : filaDaSala1());
    if (/^\/v1\/salas\/[^/]+\/horarios-livres$/.test(c)) return json(route, horariosLivres(url.searchParams.get('data') ?? '2026-11-07'));
    if (/^\/v1\/salas\/[^/]+\/bipes$/.test(c)) {
      const corpo = route.request().postDataJSON() as { codigo_etiqueta: string };
      if (corpo.codigo_etiqueta === 'ETQ-2026-018755') {
        return problema(
          route,
          409,
          'paciente-de-outra-sala',
          'A pulseira é de Juliana Sampaio Rocha, 12/04/1993, agendada para a Sala 2. O exame ficou bloqueado na Sala 1, o aparelho não recebeu a lista e a divergência foi aberta. Encaminhe a paciente à Sala 2.',
          { sala_correta: { id: SALAS[1]?.id, nome: SALAS[1]?.nome } },
        );
      }
      return json(route, { atendimento: { ...ATENDIMENTO_ANTONIO, situacao: 'em_exame' }, aparelho: APARELHO_SALA_1, worklist: 'enviada' });
    }
    if (c === '/v1/laudos/fila') return json(route, filaDeLaudos());
    if (/^\/v1\/laudos\/[^/]+$/.test(c)) {
      if (estado.semVinculo) {
        return problema(route, 403, 'sem-vinculo-de-cuidado', 'Você não tem vínculo de cuidado com este paciente. O laudo não foi aberto. Se for uma emergência, use o acesso de emergência com justificativa.');
      }
      return json(route, laudoAntonio());
    }
    if (/^\/v1\/laudos\/[^/]+\/rascunho$/.test(c)) return json(route, laudoAntonio());
    if (/^\/v1\/laudos\/[^/]+\/achados$/.test(c)) return json(route, achadosDoLaudoAntonio(laudoAntonio().id));
    if (c === '/v1/portal/laudos') return json(route, itensDoPortal());
    if (c.startsWith('/v1/portal/laudos/')) return json(route, PORTAL_LAUDO_ECO);
    if (c.startsWith('/v1/links/')) return problema(route, 410, 'link-expirado', 'Este link expirou.');
    if (c === '/v1/pedidos-desconto') return json(route, [pedidoDaSonia()]);
    if (c === '/v1/estornos') return json(route, [estornoDaSonia()]);
    if (c === '/v1/caixa/a-receber') {
      return json(
        route,
        itensDaAgenda()
          .filter((i) => typeof i.atendimento_id === 'string')
          .map((i) => ({ atendimento_id: i.atendimento_id, codigo: i.codigo, paciente: i.paciente, inicio: i.inicio, procedimentos: [{ tuss: i.exame.codigo_tuss, descricao: i.exame.nome, valor_centavos: 38000 }], recebido: false })),
      );
    }
    if (c.endsWith('/comunicacao')) return json(route, { whatsapp: true, motivo: null, mensagens: [{ modelo: 'confirmacao', situacao: 'enviada', resposta: 'confirmou', criado_em: '2026-11-05T12:00:00Z' }] });
    if (c === '/v1/configuracoes/integracoes') {
      return json(route, ['email', 'whatsapp', 'nfse', 'pix', 'assinatura', 'visualizador'].map((id, k) => ({ id, nome: `Integração ${id} com nome comprido`, para_que: 'Para que serve esta integração, em uma frase.', situacao: ['em_uso', 'incompleta', 'nao_configurada', 'desenvolvimento', 'aguardando_adaptador', 'em_uso'][k], o_que_falta: k === 1 ? 'Falta preencher: VIDACOR_WHATSAPP_NUMERO_ID.' : null, pendentes_no_outbox: k === 1 ? 3 : 0, variaveis: Array.from({ length: 8 }, (_, j) => ({ nome: `VIDACOR_NFSE_CERTIFICADO_ARQUIVO_${String(j)}`, descricao: 'Caminho do certificado A1 (.pfx) no servidor, legível só pelo núcleo.', obrigatoria: j < 6, segredo: j === 7, preenchida: j % 2 === 0 })) })));
    }
    if (c === '/v1/lista-espera') {
      return json(route, Array.from({ length: 18 }, (_, k) => ({ id: `e${String(k)}`, codigo: `ESP-${String(k)}`, posicao: k + 1, paciente: { id: `p${String(k)}`, nome: k === 0 ? 'Maria Helena Duarte de Albuquerque Cavalcanti' : `Paciente fictício ${String(k)}`, nascimento: '1968-03-12' }, exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' }, prioridade: ['urgente', 'oncologico', 'gestante', 'rotina'][k % 4], preferencia: 'qualquer', justificativa: k % 4 === 0 ? 'suspeita de valvopatia' : null, medico_solicitante: k % 4 === 0 ? 'Dr. Paulo Medeiros' : null, operadora: null, guia_codigo: null, situacao: k === 0 ? 'ofertado' : 'aguardando', desde: '2026-11-02T12:00:00Z', oferta: k === 0 ? { id: 'of1', codigo: 'OFE-1', sala_nome: 'Sala 1', inicio: '2026-11-09T13:00:00Z', expira_em: '2026-11-09T12:20:00Z' } : null })));
    }
    if (c === '/v1/aparelhos') {
      return json(route, { estudos_recebidos_hoje: 1284, estudos_sem_paciente: 1, aparelhos: Array.from({ length: 14 }, (_, k) => ({ id: `a${String(k)}`, codigo: `APA-${String(k)}`, nome: k === 0 ? 'Vivid E95 com nome de aparelho bem comprido' : `Aparelho ${String(k)}`, ae_title: `SALA${String(k)}`, modalidade: 'US', sala_id: 's', sala_nome: `Sala ${String(k)}`, ativo: k !== 3, versao: 1, situacao: k === 3 ? 'desativado' : k === 2 ? 'atencao' : 'conectado', hoje: { listas_enviadas: 12, listas_pendentes: k === 2 ? 2 : 0, listas_com_erro: k === 2 ? 2 : 0, ultimo_envio: '2026-11-06T11:30:00Z', ultimo_erro: k === 2 ? 'conexão recusada pelo aparelho' : null } })) });
    }
    if (c === '/v1/orcamentos/catalogo') {
      return json(route, { tabela_versao: 12, limite_desconto_pontos_base: 1000, exames: [{ tuss: '40901114', nome: 'Ecocardiograma transtorácico', valor_centavos: 30000 }], pacotes: [] });
    }
    if (c === '/v1/orcamentos') {
      return json(route, Array.from({ length: 20 }, (_, k) => ({ id: `o${String(k)}`, codigo: `ORC-2026-0000${String(10 + k)}`, paciente_id: 'p', paciente_nome: 'Maria Helena Duarte de Albuquerque', paciente_nascimento: '1968-03-12', itens: [], tabela_versao: 12, pacote: null, pacote_regra_versao: null, subtotal_centavos: 60000, pacote_centavos: 0, desconto_centavos: 0, desconto_pontos_base: null, desconto_motivo: null, total_centavos: 60000, pagamento_previsto: 'pix', valido_ate: '2026-11-20', situacao: k % 3 === 0 ? 'cancelado' : 'emitido', motivo_cancelamento: null, criado_em: '2026-11-06T12:00:00Z' })));
    }
    if (c === '/v1/painel-de-chamada') {
      const ch = (senha: string, destino: string, min: number) => ({ senha, nome_abreviado: 'Maria Helena D.', destino, rechamada: false, chamada_em: `2026-11-06T11:${String(min)}:00Z` });
      return json(route, { agora: { ...ch('A012', 'Sala 3 USG e Doppler com nome longo', 40), nome_abreviado: 'Antônio C. R.' }, ultimas: [ch('A011', 'Guichê 1', 36), ch('A010', 'Sala 1', 33), ch('A009', 'Sala 2', 29), ch('A008', 'Sala 6', 25)] });
    }
    if (c === '/v1/usuarios') return json(route, Array.from({ length: 25 }, (_, k) => ({ id: `u${String(k)}`, codigo: `USR-${String(k)}`, login: `usuario.${String(k)}`, nome: k === 0 ? 'Dra. Renata Alves' : `Pessoa da equipe ${String(k)}`, perfis: ['medico'], ativo: k > 0, ultimo_acesso: null, pendente: k === 0 ? { id: 'p1', tipo: 'criar', perfil: null, motivo: 'contratação', pedido_por: 'Marcos Teixeira' } : null })));
    if (c === '/v1/financeiro/resumo') return json(route, { entradas_do_mes_centavos: 41270000, saidas_do_mes_centavos: 28730000, saldo_em_contas_centavos: 53890000, a_pagar_aberto_centavos: 1500000, a_receber_aberto_centavos: 31280000, aguardando_aprovacao: 1, extrato_sem_conciliacao: 3 });
    if (c === '/v1/financeiro/titulos') return json(route, Array.from({ length: 20 }, (_, k) => ({ id: `t${String(k)}`, codigo: `TIT-2026-0000${String(k)}`, tipo: 'pagar', descricao: 'Material de consumo, nota fiscal número 1203', contraparte: 'Distribuidora de produtos hospitalares fictícia', categoria: 'fornecedor', valor_centavos: 1500000 + k, vencimento: '2026-11-20', situacao: 'aberto', aprovacoes_exigidas: 2, aprovacoes: 1, vencido: k % 3 === 0 })));
    if (c === '/v1/financeiro/contas') return json(route, []);
    if (c === '/v1/indicadores') {
      return json(route, { de: '2026-10-08', ate: '2026-11-06', agendamentos: 120, atendimentos: 104, faltas: 9, taxa_de_faltas: 0.08, receita_centavos: 4480000, recebimentos: 118, ticket_medio_centavos: 37966, tempo_medio_de_laudo_horas: 6.4,
        producao: Array.from({ length: 25 }, (_, k) => ({ exame: `Exame número ${String(k)} com nome comprido para quebrar`, atendimentos: 50 - k })),
        por_medico: Array.from({ length: 12 }, (_, k) => ({ medico: `Dr. Médico ${String(k)}`, laudos: 10 + k, horas_medias: 3 + k })),
        ocupacao: Array.from({ length: 70 }, (_, k) => ({ dia_da_semana: k % 7, hora: 7 + Math.floor(k / 7), agendamentos: (k * 7) % 13 })) });
    }
    if (c === '/v1/lastro/cadeia') {
      const evs = Array.from({ length: 30 }, (_, i) => ({ sequencia: i + 1, dominio: 'recepcao', tipo: 'evento_de_teste', registro_codigo: 'ATD-2026-000031', criado_em: '2026-11-06T11:00:00Z', origem: 'recepcao', hash: '3f9ac21b77e0', integro: true }));
      return json(route, { paciente: { id: 'p', nome: 'Sônia Maria Castro', nascimento: '1962-02-03' }, atendimento_codigo: 'ATD-2026-000031', nos: Array.from({ length: 12 }, (_, i) => ({ tipo: 'etiqueta', id: `n${String(i)}`, codigo: `ETQ-2026-0000${String(i)}`, situacao: 'valida', criado_em: '2026-11-06T11:00:00Z' })), eventos: evs, integra: true });
    }
    if (c.startsWith('/v1/faturamento/')) {
      // Lista longa de propósito: rola dentro do painel, nunca a página.
      const f = faturamentoInicial();
      const base = f.contas[1];
      if (base !== undefined) {
        for (let i = 0; i < 24; i += 1) {
          f.contas.push({ ...base, resumo: { ...base.resumo, id: `${base.resumo.id.slice(0, -2)}${String(10 + i)}`, codigo: `FAT-2026-0002${String(10 + i)}` } });
        }
      }
      if (c === '/v1/faturamento/contas') return json(route, f.contas.map((x) => x.resumo));
      if (c.startsWith('/v1/faturamento/contas/')) return json(route, f.contas.find((x) => c.endsWith(x.resumo.id)) ?? f.contas[1]);
      if (c === '/v1/faturamento/auditoria') return json(route, painelDe(f));
      if (c === '/v1/faturamento/contratos') return json(route, [CONTRATO_UNIMED]);
      if (c === '/v1/faturamento/lotes') return json(route, f.lotes);
      if (c.startsWith('/v1/faturamento/lotes/')) return json(route, { lote: f.lotes[0], contas: [f.contas[2]?.resumo] });
      if (c === '/v1/faturamento/glosas') return json(route, f.glosas);
    }
    if (c.startsWith('/v1/autorizacoes')) {
      // Lista longa de propósito: rola dentro do painel, nunca a página.
      const lista = autorizacoesIniciais();
      const base = lista[0];
      if (base !== undefined) {
        for (let i = 0; i < 20; i += 1) lista.push({ ...base, resumo: { ...base.resumo, id: `${base.resumo.id.slice(0, -2)}${String(10 + i)}` } });
      }
      if (c === '/v1/autorizacoes') return json(route, lista.map((a) => a.resumo));
      return json(route, lista.find((a) => c.endsWith(a.resumo.id)) ?? lista[0]);
    }
    if (c === '/v1/estoque/itens') {
      return json(route, [{ id: 'i1', codigo: 'ITM-CTR', nome: 'Contraste iodado não iônico 350, 100 ml', medida: 'frasco', minimo: 20, local: 'Tomografia', contraste: true, saldo_valido: 8, abaixo_do_minimo: true, lotes: [{ id: 'l1', numero: 'L2408-77', validade: '2027-03-31', saldo: 3, vencido: false }, { id: 'l2', numero: 'L2501-12', validade: '2027-09-30', saldo: 5, vencido: false }] }]);
    }
    if (c === '/v1/caixas/meu') return json(route, estado.caixaAberto ? caixaAberto() : null);
    if (c === '/v1/recebimentos') return json(route, recebimentoSonia(500, 'pix', false), 201);
    if (c.startsWith('/v1/recebimentos/')) return json(route, recebimentoSonia(500, 'pix'));
    if (c === '/v1/estudos/pendentes') {
      return json(route, [
        { id: '00000024-0000-7000-8000-000000000001', codigo: 'EST-2026-000081', accession: null, imagens: 31, recebido_em: '2026-11-06T13:05:00Z', pendencia: 'Patient ID C-0414 sem accession' },
      ]);
    }
    return route.fulfill({ status: 404, contentType: 'application/problem+json', body: '{}' });
  });
}
