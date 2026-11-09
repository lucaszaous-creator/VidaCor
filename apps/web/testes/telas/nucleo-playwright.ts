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
import { convenioUnimed, conveniosIniciais, pacotesIniciais, precosUnimed, propostaUnimed } from '../dados/convenios.ts';
import { estornoDaSonia, pedidoDaSonia } from '../dados/pendencias.ts';

export interface EstadoDaSimulacao {
  sessao: (typeof SESSOES)[keyof typeof SESSOES];
  checkinFeito: boolean;
  caixaAberto: boolean;
  contingenciaAberta: boolean;
  semVinculo: boolean;
  /** O laudo do Antônio aparece assinado (versão 1 vigente), com o PDF disponível. */
  laudoAssinado: boolean;
}

export function novoEstado(): EstadoDaSimulacao {
  return { sessao: SESSOES.recepcao, checkinFeito: false, caixaAberto: false, contingenciaAberta: false, semVinculo: false, laudoAssinado: false };
}

function json(route: Route, corpo: unknown, status = 200): Promise<void> {
  return route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(corpo) });
}

/** Começo de um PDF com os cabeçalhos do núcleo (SHA-256 e emissão); o conteúdo não importa aqui. */
function pdf(route: Route, arquivo: string, codigo: string): Promise<void> {
  return route.fulfill({
    status: 200,
    contentType: 'application/pdf',
    headers: {
      'Content-Disposition': `attachment; filename="${arquivo}"`,
      'X-Documento-Sha256': '9f2c4e7a1b3d5f60718293a4b5c6d7e8f9012345a6b7c8d9e0f1a2b3c4d5e6f7',
      'X-Documento-Codigo': codigo,
    },
    body: '%PDF-1.7',
  });
}

function laudoDaSimulacao(assinado: boolean) {
  const l = laudoAntonio();
  if (!assinado) return l;
  return {
    ...l,
    situacao: 'assinado',
    publicado_no_portal: true,
    versao_vigente: 1,
    versoes: l.versoes.map((v) => ({ ...v, situacao: 'assinada', assinada_por: 'Dr. Paulo Medeiros', assinada_em: '2026-11-06T12:04:00Z', hash_documento: 'a'.repeat(64) })),
  };
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
    if (c === '/v1/laudos/fluxo') {
      const etapas = ['a_laudar', 'em_retificacao', 'aguardando_conversa', 'assinado', 'entregue'];
      const cartoes = Array.from({ length: 40 }, (_, k) => ({ laudo_id: `l${String(k)}`, codigo: `LAU-${String(k)}`, etapa: etapas[k % 5], paciente_nome: k === 0 ? 'Maria Helena Duarte de Albuquerque Cavalcanti' : `Paciente fictício ${String(k)}`, paciente_nascimento: '1968-03-12', exame: 'Ecocardiograma transtorácico com Doppler', modalidade: 'US', medico: 'Dr. Paulo Medeiros', recebido_em: '2026-11-06T11:00:00Z', prazo_em: '2026-11-07T11:00:00Z', assinado_em: null, versao_vigente: k % 5 === 3 ? 2 : null, atrasado: k % 3 === 0 }));
      return json(route, { prazo_horas: 24, no_prazo_percentual: 87, atrasados: 14, vencem_em_2h: 5, etapas: etapas.map((etapa) => ({ etapa, quantidade: 8 })), cartoes, producao: Array.from({ length: 6 }, (_, k) => ({ medico: `Dr. Médico ${String(k)}`, assinados_hoje: k, tempo_medio_horas: 3.4, na_fila: 2 })), retificacoes: Array.from({ length: 5 }, (_, k) => ({ laudo_id: `r${String(k)}`, codigo: `LAU-R${String(k)}`, paciente_nome: 'Caio Vieira Moraes', exame: 'Raio X de tórax', versao: 2, assinada_em: '2026-11-06T13:22:00Z', motivo: 'Lateralidade corrigida de direita para esquerda.', medico: 'Dra. Cecília Souto' })) });
    }
    if (/^\/v1\/laudos\/[^/]+\/pdf$/.test(c)) return pdf(route, 'LAU-2026-004120-v1.pdf', 'DOC-2026-000007');
    if (/^\/v1\/portal\/laudos\/[^/]+\/pdf$/.test(c)) return pdf(route, 'LAU-2026-000031-v2.pdf', 'DOC-2026-000008');
    if (c === '/v1/lastro/dossie') return pdf(route, 'DOS-2026-000003.pdf', 'DOS-2026-000003');
    if (/^\/v1\/laudos\/[^/]+$/.test(c)) {
      if (estado.semVinculo) {
        return problema(route, 403, 'sem-vinculo-de-cuidado', 'Você não tem vínculo de cuidado com este paciente. O laudo não foi aberto. Se for uma emergência, use o acesso de emergência com justificativa.');
      }
      return json(route, laudoDaSimulacao(estado.laudoAssinado));
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
    if (/^\/v1\/pacientes\/[^/]+\/visao$/.test(c)) {
      const ag = (k: number, futuro: boolean) => ({ id: `a${String(k)}`, codigo: `AGD-${String(k)}`, inicio: futuro ? '2026-11-12T11:00:00Z' : '2025-05-10T12:00:00Z', exame: k === 0 ? 'Ecocardiograma transtorácico com Doppler colorido e strain' : `Exame ${String(k)}`, sala: 'Sala 1', medico: 'Dr. Paulo Medeiros', situacao: futuro ? 'confirmado' : 'concluido' });
      return json(route, { identificacao: { id: 'p', codigo: 'PAC-2026-000004', nome: 'Maria Helena Duarte de Albuquerque Cavalcanti', nome_social: null, nascimento: '1968-03-12', sexo: 'F', cpf_mascarado: '***.***.***-25', celular: '21900000017', email: 'maria.helena.duarte.albuquerque@exemplo.invalid', alergias: ['dipirona', 'contraste iodado'], nega_alergias: false }, proximos: Array.from({ length: 3 }, (_, k) => ag(k, true)), historico: Array.from({ length: 25 }, (_, k) => ag(k, false)), faltas: 1, exames_externos: 2 });
    }
    if (c === '/v1/qualidade') {
      return json(route, { nc_abertas: 9, nc_vencidas: 2, eventos_no_mes: 14, quase_erros_no_mes: 3, graves_no_mes: 1, anvisa_pendentes: 1, nao_conformidades: Array.from({ length: 12 }, (_, k) => ({ id: `n${String(k)}`, codigo: `NCF-2026-0000${String(10 + k)}`, descricao: 'Nobreak da sala 6 sem manutenção preventiva há oito meses, com risco de parada no meio do exame.', classificacao: 'equipamento', gravidade: 'moderada', responsavel: 'Patrícia Lemos', prazo_em: '2026-12-01', situacao: k % 4 === 0 ? 'encerrada' : 'em_acao', eficaz: k % 4 === 0 ? true : null, vencida: k === 1, aberta_por: 'Equipe TI', aberta_em: '2026-11-06T12:00:00Z', versao: 2, andamentos: [{ texto: 'Chamado aberto com o fornecedor do nobreak.', situacao_nova: 'em_acao', por: 'Patrícia Lemos', em: '2026-11-06T13:00:00Z' }] })), eventos: [] });
    }
    if (c === '/v1/ouvidoria') {
      return json(route, { abertas: 12, vencem_hoje: 2, no_prazo_percentual: 94, manifestacoes: Array.from({ length: 18 }, (_, k) => ({ id: `m${String(k)}`, codigo: `OUV-2026-0000${String(10 + k)}`, tipo: ['reclamacao', 'elogio', 'sugestao', 'denuncia', 'lgpd'][k % 5], canal: 'presencial', quem: k === 0 ? 'Maria Helena Duarte de Albuquerque Cavalcanti' : `Pessoa fictícia ${String(k)}`, nascimento: '1968-03-12', anonima: false, relato: 'Esperou cinquenta minutos na recepção sem nenhum aviso sobre o atraso do aparelho de Raio X. '.repeat(4), recebida_em: '2026-11-06', prazo_em: '2026-11-13', situacao: k < 12 ? 'aberta' : 'respondida', vencida: k === 1, resposta: k < 12 ? null : 'Resposta enviada.', respondida_em: null })) });
    }
    if (c === '/v1/inicio') {
      return json(route, { data: '2026-11-06', exames_agendados: 141, realizados: 112, aguardando_agora: 16, faltas: 11, laudos_a_liberar: 211, retificacoes_em_rascunho: 3, reservas_sem_guia: 12, proximos: Array.from({ length: 6 }, (_, k) => ({ agendamento_id: `a${String(k)}`, inicio: '2026-11-06T12:00:00Z', nome: k === 0 ? 'Maria Helena Duarte de Albuquerque Cavalcanti' : `Paciente fictício ${String(k)}`, nascimento: '1968-03-12', exame: 'Ecocardiograma transtorácico com Doppler colorido', sala: 'Sala 1', situacao: k === 1 ? 'reserva_provisoria' : 'confirmado' })), ocupacao: Array.from({ length: 7 }, (_, k) => ({ sala: k === 6 ? 'Posto MAPA e Holter' : `Sala ${String(k + 1)}`, minutos_marcados: 90 * k, minutos_abertos: 720, percentual: Math.min(100, Math.round((90 * k * 100) / 720)) })), caixa: { recebido_centavos: 378400, pix_centavos: 164000, cartao_centavos: 120400, dinheiro_centavos: 94000, aguardando_pagamento: 3 } });
    }
    if (c === '/v1/configuracoes/integracoes') {
      return json(route, ['email', 'whatsapp', 'nfse', 'pix', 'assinatura', 'visualizador'].map((id, k) => ({ id, nome: `Integração ${id} com nome comprido`, para_que: 'Para que serve esta integração, em uma frase.', situacao: ['em_uso', 'incompleta', 'nao_configurada', 'desenvolvimento', 'aguardando_adaptador', 'em_uso'][k], o_que_falta: k === 1 ? 'Falta preencher: VIDACOR_WHATSAPP_NUMERO_ID.' : null, pendentes_no_outbox: k === 1 ? 3 : 0, variaveis: Array.from({ length: 8 }, (_, j) => ({ nome: `VIDACOR_NFSE_CERTIFICADO_ARQUIVO_${String(j)}`, descricao: 'Caminho do certificado A1 (.pfx) no servidor, legível só pelo núcleo.', obrigatoria: j < 6, segredo: j === 7, preenchida: j % 2 === 0 })) })));
    }
    if (c === '/v1/lista-espera') {
      return json(route, Array.from({ length: 18 }, (_, k) => ({ id: `e${String(k)}`, codigo: `ESP-${String(k)}`, posicao: k + 1, paciente: { id: `p${String(k)}`, nome: k === 0 ? 'Maria Helena Duarte de Albuquerque Cavalcanti' : `Paciente fictício ${String(k)}`, nascimento: '1968-03-12' }, exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' }, prioridade: ['urgente', 'oncologico', 'gestante', 'rotina'][k % 4], preferencia: 'qualquer', justificativa: k % 4 === 0 ? 'suspeita de valvopatia' : null, medico_solicitante: k % 4 === 0 ? 'Dr. Paulo Medeiros' : null, operadora: null, guia_codigo: null, situacao: k === 0 ? 'ofertado' : 'aguardando', desde: '2026-11-02T12:00:00Z', oferta: k === 0 ? { id: 'of1', codigo: 'OFE-1', sala_nome: 'Sala 1', inicio: '2026-11-09T13:00:00Z', expira_em: '2026-11-09T12:20:00Z' } : null })));
    }
    if (c === '/v1/corpo-clinico') {
      const pedido = { id: 'pc1', codigo: 'PCC-2026-000001', tipo: 'registro', situacao: 'aguardando', registro: { uf: 'SP', numero: '6543210', rqe: '99887', especialidade: 'Radiologia e Diagnóstico por Imagem' }, habilitacao: null, motivo: 'Contratação para a tomografia e a ressonância da unidade Centro', pedido_por: 'Helena Prado de Albuquerque', pedido_por_id: 'outra', pedido_em: '2026-11-06T20:20:00Z', decidido_por: null, decidido_em: null, motivo_decisao: null };
      return json(route, Array.from({ length: 24 }, (_, k) => ({ id: `m${String(k)}`, codigo: `USR-medico.${String(k)}`, nome: k === 0 ? 'Dra. Maria Cecília de Albuquerque Cavalcanti Souto' : `Dr. Médico Fictício ${String(k)}`, ativo: k !== 5, situacao: k === 0 ? 'aguardando_conferencia' : k === 5 ? 'inativo' : k % 4 === 0 ? 'sem_registro' : 'ativo', registro: k % 4 === 0 ? null : { conselho: 'CRM', uf: 'SP', numero: String(100000 + k), rqe: String(7000 + k), especialidade: 'Cardiologia e Ecocardiografia', desde: '2026-01-03T12:00:00Z' }, habilitacao: { realiza: ['US', 'CT', 'DX', 'MR', 'MG'], lauda: ['US', 'CT', 'DX', 'MR', 'MG'], desde: '2026-01-03T12:00:00Z' }, pendentes: k === 0 ? [pedido] : [] })));
    }
    if (/^\/v1\/corpo-clinico\/[^/]+\/historico$/.test(c)) {
      return json(route, Array.from({ length: 12 }, (_, k) => ({ id: `h${String(k)}`, codigo: `PCC-2026-0000${String(10 + k)}`, tipo: k % 2 === 0 ? 'registro' : 'habilitacao', situacao: k === 3 ? 'recusado' : 'aplicado', registro: k % 2 === 0 ? { uf: 'SP', numero: String(100000 + k), rqe: '7788', especialidade: 'Cardiologia e Ecocardiografia' } : null, habilitacao: k % 2 === 1 ? { realiza: ['US', 'CT'], lauda: ['US', 'CT', 'DX'] } : null, motivo: 'Conferência anual do cadastro com o documento do conselho regional', pedido_por: 'Patrícia Lemos', pedido_por_id: 'p', pedido_em: '2026-03-12T12:00:00Z', decidido_por: 'Helena Prado de Albuquerque', decidido_em: '2026-03-12T14:00:00Z', motivo_decisao: 'Conferido no portal do CFM com o número e a especialidade' })));
    }
    if (c === '/v1/catalogo/procedimentos') {
      return json(route, { procedimentos: Array.from({ length: 60 }, (_, k) => ({ tuss: String(40901100 + k), nome: k === 0 ? 'Ecocardiograma transtorácico com Doppler colorido e strain bidimensional' : `Exame fictício ${String(k)}`, modalidade: ['US', 'CT', 'DX', 'MR'][k % 4], duracao_min: 30, requer_contraste: k % 7 === 0, ativo: k % 9 !== 0 || k === 0, versao: 1 + (k % 3), preco_centavos: k % 5 === 0 ? null : 38000 + k * 100, proximo_preco_centavos: k % 5 === 0 ? null : 40000 + k * 100 })), tabelas: [{ id: 't13', versao: 13, vigente_desde: '2027-01-01', descricao: 'Particular 2027 com reajuste anual', motivo: 'Reajuste anual aprovado pela diretoria', itens: 48, publicada_por: 'Patrícia Lemos', publicada_em: '2026-11-05T12:00:00Z', situacao: 'futura' }, { id: 't12', versao: 12, vigente_desde: '2026-01-01', descricao: 'Particular 2026', motivo: null, itens: 45, publicada_por: null, publicada_em: '2025-12-20T12:00:00Z', situacao: 'vigente' }] });
    }
    if (/^\/v1\/catalogo\/procedimentos\/[^/]+\/historico$/.test(c)) {
      const e = (ativo: boolean, d: number) => ({ nome: 'Ecocardiograma transtorácico', modalidade: 'US', duracao_min: d, requer_contraste: false, ativo });
      return json(route, Array.from({ length: 10 }, (_, k) => ({ codigo: `PRM-2026-0000${String(10 + k)}`, tipo: k === 9 ? 'incluir' : 'alterar', motivo: 'Ajuste da duração pela média real dos últimos 90 dias na agenda da Sala 1', versao_do_procedimento: 10 - k, antes: k === 9 ? null : e(true, 30), depois: e(true, 40), quem: 'Patrícia Lemos', em: '2026-10-01T12:00:00Z' })));
    }
    if (c === '/v1/configuracoes/cabecalho') {
      const v = (n: number) => ({ numero: n, nome: 'VidaCor Imagem Centro, unidade de diagnóstico por imagem', razao_social: 'VidaCor Diagnóstico por Imagem e Cardiologia Não Invasiva Ltda.', cnpj: '11.222.333/0001-81', endereco: 'Avenida Fictícia das Palmeiras, 1234, sala 56, Centro, São Paulo, SP', telefone: '(11) 3000-0000', site: 'vidacorimagem.com.br', responsavel_nome: 'Dra. Maria Cecília de Albuquerque Souto', responsavel_crm: 'CRM-SP 123456', responsavel_tecnico: 'Dra. Maria Cecília de Albuquerque Souto, CRM-SP 123456', motivo: n === 1 ? null : 'Mudança de endereço da unidade', publicado_por: 'Patrícia Lemos', desde: '2026-10-01T12:00:00Z' });
      return json(route, { vigente: v(6), versoes: [6, 5, 4, 3, 2, 1].map(v) });
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
    // Convênios e pacotes: listas longas de propósito (rolam dentro do painel, nunca a página).
    if (c === '/v1/convenios') {
      const base = conveniosIniciais(true);
      const extras = Array.from({ length: 18 }, (_, k) => ({ ...base[1], id: `cvn${String(k)}`, codigo: `CVN-2026-0001${String(10 + k)}`, operadora: k === 0 ? 'Caixa de Assistência dos Servidores Públicos Municipais Fictícia' : `Operadora fictícia ${String(k)}` }));
      return json(route, [...base, ...extras]);
    }
    if (/^\/v1\/convenios\/[^/]+\/precos$/.test(c)) return json(route, precosUnimed(40));
    if (/^\/v1\/convenios\/[^/]+\/previa-de-reajuste$/.test(c)) {
      const p = propostaUnimed();
      return json(route, { base_versao: 14, pontos_base: 520, afetados: 142, total_antes_centavos: p.total_antes_centavos, total_depois_centavos: p.total_depois_centavos, maiores: p.maiores });
    }
    if (/^\/v1\/convenios\/[^/]+$/.test(c)) return json(route, convenioUnimed(true));
    if (c === '/v1/pacotes') return json(route, pacotesIniciais(16));
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
