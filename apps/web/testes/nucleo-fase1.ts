/**
 * Núcleo simulado (MSW) das rotas da Fase 1, conforme o docs/30. Serve só para conferir o que a TELA
 * mostra e manda; a prova das regras (cálculo da TFG e do total, concorrência, RLS) é dos testes de
 * integração do núcleo com banco real (G4). Cada caso de erro do contrato é provocado pelo teste com
 * `servidor.use(...)` ou pelos dados de cada rota.
 */
import type {
  AtendimentoFase1,
  Caixa,
  Contingencia,
  Guia,
  Laudo,
  PedidoAgendamento,
  PedidoContingencia,
  PedidoFicha,
  PedidoGuia,
  PedidoRecebimento,
  PedidoRecibo,
  PedidoTriagem,
  Triagem,
} from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { itensDaAgenda, SALAS, uuid } from './dados/ficticios';
import {
  APARELHO_SALA_1,
  APARELHO_SALA_6,
  ATENDIMENTO_ANTONIO,
  ATENDIMENTO_VERA,
  caixaAberto,
  contingenciaAberta,
  filaDaSala1,
  filaDaSala6,
  filaDeLaudos,
  horariosLivres,
  itensDoPortal,
  JULIANA,
  laudoAntonio,
  LAUDO_ANTONIO_ID,
  MARIA_HELENA,
  PACIENTES,
  PORTAL_LAUDO_DOPPLER,
  PORTAL_LAUDO_ECO,
  PULSEIRAS,
  recebimentoSonia,
  SALA_1,
  SALA_2,
  SALA_6,
  TRIAGEM_LIBERADA,
  TRIAGEM_PRECISA_APROVACAO,
} from './dados/fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';

export interface PedidoRegistrado {
  rota: string;
  chave: string | null;
  csrf: string | null;
  estacao: string | null;
  corpo: unknown;
}

interface EstornoSimulado {
  id: string;
  situacao: string;
  motivo: string;
  pedido_por: string;
}

export interface EstadoFase1 {
  /** Atos que respondem `401 reautenticacao-necessaria` até o teste tirar daqui. */
  exigirReautenticacao: Set<string>;
  pedidos: PedidoRegistrado[];
  bipados: Set<string>;
  triagemVera: Triagem | null;
  etiquetaAntonio: string;
  laudo: Laudo;
  acessoDeEmergencia: boolean;
  caixa: Caixa | null;
  estornos: Map<string, EstornoSimulado>;
  contingencia: Contingencia | null;
  fichas: Map<string, AtendimentoFase1>;
  divergenciasAbertas: string[];
  guias: Map<string, Guia>;
  /** Último recebimento criado; o PIX só aparece na consulta seguinte (PSP pelo outbox). */
  recebimento: { pb: number; forma: string; consultas: number } | null;
  /** O PSP já devolveu a cobrança PIX (o teste segura para ver o estado intermediário). */
  pspRespondeu: boolean;
  estudosConciliados: Set<string>;
}

export const fase1: EstadoFase1 = novoEstado();

function novoEstado(): EstadoFase1 {
  return {
    exigirReautenticacao: new Set(),
    pedidos: [],
    bipados: new Set(),
    triagemVera: null,
    etiquetaAntonio: PULSEIRAS.antonio,
    laudo: laudoAntonio(),
    acessoDeEmergencia: false,
    caixa: null,
    estornos: new Map(),
    contingencia: null,
    fichas: new Map(),
    divergenciasAbertas: [],
    guias: new Map(),
    recebimento: null,
    pspRespondeu: true,
    estudosConciliados: new Set(),
  };
}

export function reiniciarFase1(): void {
  Object.assign(fase1, novoEstado());
}

/** Pedidos de uma rota (o teste confere corpo, chave e cabeçalhos). */
export function pedidosDe(rota: string): PedidoRegistrado[] {
  return fase1.pedidos.filter((p) => p.rota === rota);
}

async function registrar(rota: string, request: Request): Promise<unknown> {
  let corpo: unknown;
  try {
    corpo = await request.clone().json();
  } catch {
    corpo = null;
  }
  fase1.pedidos.push({
    rota,
    chave: request.headers.get('Idempotency-Key'),
    csrf: request.headers.get('X-CSRF-Token'),
    estacao: request.headers.get('X-Estacao'),
    corpo,
  });
  return corpo;
}

function reautenticacao(ato: string) {
  if (!fase1.exigirReautenticacao.has(ato)) {
    return null;
  }
  return problema(
    401,
    'reautenticacao-necessaria',
    'Este ato pede a confirmação da sua senha e do segundo fator, porque o último login tem mais de 5 minutos. Nada foi gravado.',
  );
}

function temPerfil(...perfis: string[]): boolean {
  return estado.sessao.usuario.perfis.some((p) => perfis.includes(p));
}

const sessaoEh = (nome: string): boolean => estado.sessao.usuario.nome === nome;

function semPermissao(oQue: string) {
  return problema(403, 'sem-permissao', `Seu perfil não pode ${oQue}. Nada foi feito. Se precisar, peça à coordenação.`);
}

const versaoAssinada = (texto: string, numero: number, motivo: string | null) => ({
  numero,
  texto,
  situacao: 'assinada' as const,
  motivo,
  assinada_por: 'Dr. Paulo Medeiros',
  assinada_em: '2026-11-06T12:04:00Z',
  hash_documento: `sha256:${String(numero).repeat(8)}`,
});

export const manipuladoresFase1 = [
  /* --------------------------------------- Recepção e etiquetas ------------------------------- */
  http.get('*/v1/atendimentos/:id', ({ params }) => {
    if (params.id === ATENDIMENTO_ANTONIO.id) {
      return HttpResponse.json({
        ...ATENDIMENTO_ANTONIO,
        pulseira: { codigo: fase1.etiquetaAntonio, situacao: 'valida' },
        laudo_situacao: fase1.laudo.situacao,
      });
    }
    if (params.id === ATENDIMENTO_VERA.id) {
      return HttpResponse.json({ ...ATENDIMENTO_VERA, triagem: fase1.triagemVera });
    }
    return undefined;
  }),
  http.post('*/v1/etiquetas/:codigo/reimpressao', async ({ request, params }) => {
    const corpo = (await registrar('reimpressao', request)) as { motivo?: string };
    if (!temPerfil('recepcao', 'supervisor_recepcao')) {
      return semPermissao('reimprimir etiqueta');
    }
    if ((corpo.motivo ?? '').trim() === '') {
      return problema(422, 'motivo-obrigatorio', 'A reimpressão precisa de motivo. Nada foi impresso. Escolha o motivo e repita.');
    }
    const codigo = String(params.codigo);
    if (codigo !== fase1.etiquetaAntonio) {
      return problema(409, 'etiqueta-ja-invalidada', `A etiqueta ${codigo} já foi invalidada. Nada foi impresso. Reimprima a partir da etiqueta válida.`);
    }
    const nova = 'ETQ-2026-018790';
    fase1.etiquetaAntonio = nova;
    return HttpResponse.json(
      {
        nova: { codigo: nova, tipo: 'pulseira', situacao: 'valida', motivo_invalidacao: null, substituida_por: null },
        invalidada: { codigo, tipo: 'pulseira', situacao: 'invalidada', motivo_invalidacao: corpo.motivo, substituida_por: nova },
      },
      { status: 201 },
    );
  }),
  http.post('*/v1/pacientes/:id/correcao', async ({ request, params }) => {
    const corpo = (await registrar('correcao', request)) as { nascimento: string; motivo: string };
    const re = reautenticacao('correcao');
    if (re !== null) return re;
    if (corpo.motivo.trim() === '') {
      return problema(422, 'motivo-obrigatorio', 'A correção precisa de motivo. Nada foi alterado.');
    }
    const paciente = PACIENTES.find((p) => p.id === params.id);
    return HttpResponse.json({
      paciente: { ...paciente, nascimento: corpo.nascimento },
      versao: 2,
      etiquetas_invalidadas: [fase1.etiquetaAntonio],
    });
  }),
  http.get('*/v1/pacientes', ({ request }) => {
    const texto = (new URL(request.url).searchParams.get('busca') ?? '').toLocaleLowerCase('pt-BR');
    return HttpResponse.json(PACIENTES.filter((p) => p.nome.toLocaleLowerCase('pt-BR').includes(texto)).slice(0, 20));
  }),

  /* --------------------------------------------- Sala ----------------------------------------- */
  http.get('*/v1/salas/:id/fila', ({ params }) => {
    if (params.id === SALA_6.id) return HttpResponse.json(filaDaSala6(fase1.triagemVera));
    if (params.id === SALA_1.id) return HttpResponse.json(filaDaSala1());
    return HttpResponse.json({ sala: SALAS.find((s) => s.id === params.id), itens: [] });
  }),
  http.post('*/v1/salas/:id/bipes', async ({ request, params }) => {
    const corpo = (await registrar('bipe', request)) as { codigo_etiqueta: string };
    const codigo = corpo.codigo_etiqueta;
    if (codigo === PULSEIRAS.invalidada || (codigo === PULSEIRAS.antonio && fase1.etiquetaAntonio !== PULSEIRAS.antonio)) {
      return problema(
        409,
        'etiqueta-substituida',
        `A etiqueta ${codigo} foi substituída por reimpressão. O exame não foi liberado e o aparelho não recebeu nada. Peça a pulseira nova à recepção.`,
      );
    }
    if (codigo === PULSEIRAS.juliana && params.id === SALA_1.id) {
      return problema(
        409,
        'paciente-de-outra-sala',
        'A pulseira é de Juliana Sampaio Rocha, 12/04/1993, agendada para a Sala 2. O exame ficou bloqueado na Sala 1, o aparelho não recebeu a lista e a divergência foi aberta. Encaminhe a paciente à Sala 2.',
        { sala_correta: { id: SALA_2.id, nome: SALA_2.nome }, divergencia: { id: uuid('00000023', 1), codigo: 'DIV-2026-000029' } },
      );
    }
    if (codigo === PULSEIRAS.vera) {
      if (fase1.triagemVera === null) {
        return problema(422, 'triagem-ausente', 'O exame com contraste não tem triagem registrada. Nada foi enviado ao aparelho. Registre a triagem antes de iniciar.');
      }
      if (fase1.triagemVera.resultado === 'precisa_aprovacao_medica' && fase1.triagemVera.conduta === null) {
        return problema(409, 'conduta-medica-pendente', 'A triagem pede aprovação médica e a conduta ainda não foi registrada. Nada foi enviado ao aparelho. Aguarde a médica registrar a conduta.');
      }
      return HttpResponse.json({ atendimento: { ...ATENDIMENTO_VERA, situacao: 'em_exame' }, aparelho: APARELHO_SALA_6, worklist: 'enviada' });
    }
    if (codigo === PULSEIRAS.antonio || codigo === fase1.etiquetaAntonio) {
      const ja = fase1.bipados.has(codigo);
      fase1.bipados.add(codigo);
      return HttpResponse.json({
        atendimento: { ...ATENDIMENTO_ANTONIO, situacao: 'em_exame' },
        aparelho: APARELHO_SALA_1,
        worklist: ja ? 'ja_enviada' : 'enviada',
      });
    }
    return problema(404, 'etiqueta-nao-encontrada', `A etiqueta ${codigo} não foi encontrada. Nada foi enviado ao aparelho. Confira o código ou bipe de novo.`);
  }),

  /* ------------------------------------------- Triagem ---------------------------------------- */
  http.post('*/v1/atendimentos/:id/triagem', async ({ request }) => {
    const corpo = (await registrar('triagem', request)) as PedidoTriagem;
    if (!temPerfil('tecnico', 'enfermagem')) {
      return semPermissao('registrar triagem');
    }
    // O núcleo calcula pela CKD-EPI 2021 (FC-TRI-07); aqui só se devolve o resultado combinado.
    fase1.triagemVera =
      corpo.creatinina_mg_dl === '1.4' || corpo.reacao_previa_contraste ? { ...TRIAGEM_PRECISA_APROVACAO } : { ...TRIAGEM_LIBERADA };
    return HttpResponse.json(fase1.triagemVera, { status: 201 });
  }),
  http.post('*/v1/atendimentos/:id/triagem/conduta', async ({ request }) => {
    const corpo = (await registrar('conduta', request)) as { conduta: string };
    if (!temPerfil('medico')) {
      return problema(403, 'sem-permissao', 'Só a médica com vínculo de cuidado registra a conduta. Nada foi gravado. Peça à médica responsável.');
    }
    const re = reautenticacao('conduta');
    if (re !== null) return re;
    fase1.triagemVera = {
      ...(fase1.triagemVera ?? TRIAGEM_PRECISA_APROVACAO),
      resultado: 'liberado_com_ressalvas',
      exame_liberado: true,
      conduta: { texto: corpo.conduta, medico_id: estado.sessao.usuario.id, registrada_em: '2026-11-06T11:58:00Z' },
    };
    return HttpResponse.json(fase1.triagemVera);
  }),

  /* ---------------------------------- Agendamento com guia ------------------------------------ */
  http.get('*/v1/salas/:id/horarios-livres', ({ request }) => {
    return HttpResponse.json(horariosLivres(new URL(request.url).searchParams.get('data') ?? '2026-11-07'));
  }),
  http.post('*/v1/guias', async ({ request }) => {
    const corpo = (await registrar('guia', request)) as PedidoGuia;
    const guia: Guia = {
      id: uuid('0000001f', fase1.guias.size + 1),
      codigo: `GUI-2026-${String(fase1.guias.size + 1).padStart(6, '0')}`,
      numero: corpo.numero,
      operadora: corpo.operadora,
      validade: corpo.validade,
      situacao: 'disponivel',
      procedimentos: corpo.procedimentos.map((p) => ({ tuss: p.tuss, autorizado: p.quantidade, saldo: p.quantidade })),
      em_uso_por: null,
    };
    fase1.guias.set(guia.id, guia);
    return HttpResponse.json(guia, { status: 201 });
  }),
  http.post('*/v1/agendamentos', async ({ request }) => {
    const corpo = (await registrar('agendamento', request)) as PedidoAgendamento;
    if (!temPerfil('recepcao', 'supervisor_recepcao', 'coordenacao', 'gerente')) {
      return semPermissao('agendar');
    }
    const paciente = PACIENTES.find((p) => p.id === corpo.paciente_id) ?? PACIENTES[0];
    const sala = SALAS.find((s) => s.id === corpo.sala_id) ?? SALA_1;
    return HttpResponse.json(
      {
        id: uuid('00000020', 1),
        codigo: 'AG-2611-4321',
        inicio: corpo.inicio,
        fim: corpo.inicio.replace(/T(\d{2}):(\d{2})/, (_m, h: string, m: string) => (m === '00' ? `T${h}:30` : `T${String(Number(h) + 1).padStart(2, '0')}:00`)),
        sala,
        paciente,
        situacao: corpo.reserva_provisoria ? 'reserva_provisoria' : 'confirmado',
        expira_em: corpo.reserva_provisoria ? '2026-11-09T12:16:00Z' : null,
      },
      { status: 201 },
    );
  }),

  http.post('*/v1/agendamentos/:id/guia', async ({ request, params }) => {
    const corpo = (await registrar('vincular-guia', request)) as { guia_id: string };
    if (!fase1.guias.has(corpo.guia_id)) {
      return problema(422, 'guia-invalida', 'A guia informada não foi encontrada. Nada foi confirmado.');
    }
    return HttpResponse.json({
      id: String(params.id),
      codigo: 'AG-2611-4321',
      inicio: '2026-11-07T09:00:00-03:00',
      fim: '2026-11-07T09:30:00-03:00',
      sala: SALA_1,
      paciente: PACIENTES[0],
      situacao: 'confirmado',
      expira_em: null,
    });
  }),

  /* --------------------------------------------- Laudo ---------------------------------------- */
  http.get('*/v1/laudos/fila', () => {
    if (!temPerfil('medico')) return semPermissao('laudar');
    return HttpResponse.json(filaDeLaudos());
  }),
  http.get('*/v1/laudos/:id', ({ params }) => {
    if (temPerfil('recepcao', 'ti', 'caixa')) {
      return problema(403, 'sem-permissao', 'Seu perfil não lê o conteúdo de laudos. Nada foi aberto. A situação do laudo aparece no atendimento.');
    }
    if (sessaoEh('Dr. Marcos Freire') && !fase1.acessoDeEmergencia) {
      return problema(
        403,
        'sem-vinculo-de-cuidado',
        'Você não tem vínculo de cuidado com este paciente. O laudo não foi aberto. Se for uma emergência, use o acesso de emergência com justificativa.',
      );
    }
    if (params.id !== LAUDO_ANTONIO_ID) {
      return problema(404, 'nao-encontrado', 'Laudo não encontrado.');
    }
    return HttpResponse.json(fase1.laudo);
  }),
  http.put('*/v1/laudos/:id/rascunho', async ({ request }) => {
    const corpo = (await registrar('rascunho', request)) as { texto: string; versao_numero: number };
    if (fase1.laudo.situacao === 'assinado') {
      return problema(409, 'laudo-assinado', 'O laudo já está assinado e não pode ser alterado. Nada foi salvo. Para corrigir, crie uma retificação com motivo.');
    }
    fase1.laudo = {
      ...fase1.laudo,
      versoes: fase1.laudo.versoes.map((v) => (v.numero === corpo.versao_numero ? { ...v, texto: corpo.texto } : v)),
    };
    return HttpResponse.json(fase1.laudo);
  }),
  http.post('*/v1/laudos/:id/assinatura', async ({ request }) => {
    await registrar('assinatura', request);
    if (!sessaoEh('Dr. Paulo Medeiros')) {
      return problema(403, 'sem-permissao', 'Só o médico atribuído assina este laudo. Nada foi assinado.');
    }
    const re = reautenticacao('assinatura');
    if (re !== null) return re;
    const rascunho = fase1.laudo.versoes.find((v) => v.situacao === 'rascunho');
    if (rascunho === undefined) {
      return problema(409, 'assinatura-concorrente', 'O laudo acabou de ser assinado em outra estação. Nada foi assinado de novo. Abra a versão vigente.');
    }
    const versoes = fase1.laudo.versoes.map((v) =>
      v.numero === rascunho.numero
        ? versaoAssinada(v.texto, v.numero, v.motivo ?? null)
        : v.situacao === 'assinada'
          ? { ...v, situacao: 'substituida' as const }
          : v,
    );
    fase1.laudo = {
      ...fase1.laudo,
      situacao: 'assinado',
      versao_vigente: rascunho.numero,
      versoes,
      publicado_no_portal: !fase1.laudo.conversar_antes,
    };
    return HttpResponse.json(fase1.laudo);
  }),
  http.post('*/v1/laudos/:id/retificacao', async ({ request }) => {
    const corpo = (await registrar('retificacao', request)) as { texto: string; motivo: string };
    const re = reautenticacao('retificacao');
    if (re !== null) return re;
    if (corpo.motivo.trim() === '') {
      return problema(422, 'motivo-obrigatorio', 'A retificação precisa de motivo. Nada foi criado. Escreva o motivo e repita.');
    }
    const numero = fase1.laudo.versoes.length + 1;
    fase1.laudo = {
      ...fase1.laudo,
      situacao: 'retificacao_em_rascunho',
      versoes: [
        ...fase1.laudo.versoes,
        { numero, texto: corpo.texto, situacao: 'rascunho', motivo: corpo.motivo, assinada_por: null, assinada_em: null, hash_documento: null },
      ],
    };
    return HttpResponse.json(fase1.laudo, { status: 201 });
  }),
  http.post('*/v1/laudos/:id/conversar-antes', async ({ request }) => {
    const corpo = (await registrar('conversar-antes', request)) as { marcado: boolean };
    fase1.laudo = { ...fase1.laudo, conversar_antes: corpo.marcado };
    return HttpResponse.json(fase1.laudo);
  }),
  http.post('*/v1/laudos/:id/liberacao', async ({ request }) => {
    await registrar('liberacao', request);
    if (fase1.laudo.situacao !== 'assinado') {
      return problema(409, 'laudo-nao-assinado', 'O laudo ainda não foi assinado e não vai ao portal. Nada foi publicado. Assine antes de liberar.');
    }
    fase1.laudo = { ...fase1.laudo, publicado_no_portal: true };
    return HttpResponse.json(fase1.laudo);
  }),
  http.post('*/v1/laudos/:id/acesso-emergencia', async ({ request }) => {
    const corpo = (await registrar('emergencia', request)) as { justificativa: string };
    const re = reautenticacao('emergencia');
    if (re !== null) return re;
    if (corpo.justificativa.trim().length < 20) {
      return problema(422, 'justificativa-curta', 'A justificativa precisa de pelo menos 20 caracteres. Nada foi aberto.');
    }
    fase1.acessoDeEmergencia = true;
    return HttpResponse.json({ valido_ate: '2026-11-06T13:10:00Z' }, { status: 201 });
  }),

  http.post('*/v1/laudos/:id/publicacao', async ({ request }) => {
    await registrar('publicacao', request);
    if (fase1.laudo.situacao !== 'assinado') {
      return problema(409, 'laudo-nao-assinado', 'O laudo ainda não foi assinado e não vai ao portal. Nada foi publicado.');
    }
    fase1.laudo = { ...fase1.laudo, publicado_no_portal: true };
    return HttpResponse.json({ publicado: true });
  }),
  http.post('*/v1/laudos/:id/links', async ({ request }) => {
    const corpo = (await registrar('link', request)) as { horas: number };
    return HttpResponse.json({ token: 'tk-ficticio-unico-8f2a', expira_em: corpo.horas === 24 ? '2026-11-07T12:00:00Z' : '2026-11-08T12:00:00Z' }, { status: 201 });
  }),

  /* -------------------------------------------- Estudos --------------------------------------- */
  http.get('*/v1/estudos/pendentes', () => {
    if (!temPerfil('medico')) return semPermissao('conciliar estudos');
    return HttpResponse.json(
      [
        { id: uuid('00000024', 1), codigo: 'EST-2026-000081', accession: null, imagens: 31, recebido_em: '2026-11-06T13:05:00Z', pendencia: 'Patient ID C-0414 sem accession' },
        { id: uuid('00000024', 2), codigo: 'EST-2026-000082', accession: 'ACC-2026-099999', imagens: 12, recebido_em: '2026-11-06T13:20:00Z', pendencia: 'Accession desconhecido' },
      ].filter((e) => !fase1.estudosConciliados.has(e.id)),
    );
  }),
  http.post('*/v1/estudos/:id/conciliacao', async ({ request, params }) => {
    const corpo = (await registrar('conciliacao', request)) as { atendimento_id: string; nome: string; nascimento: string };
    const item = itensDaAgenda().find((i) => i.atendimento_id === corpo.atendimento_id);
    if (item?.paciente.nascimento !== corpo.nascimento || item.paciente.nome !== corpo.nome.trim()) {
      return problema(422, 'identificacao-nao-confere', 'O nome ou o nascimento não confere com o atendimento escolhido. O estudo não foi conciliado. Confira os dois identificadores.');
    }
    fase1.estudosConciliados.add(String(params.id));
    return HttpResponse.json({ conciliado: true });
  }),

  /* -------------------------------------------- Portal ---------------------------------------- */
  http.get('*/v1/portal/laudos', () => HttpResponse.json(itensDoPortal())),
  http.get('*/v1/portal/laudos/:id', ({ params }) => {
    const laudo = [PORTAL_LAUDO_ECO, PORTAL_LAUDO_DOPPLER].find((l) => l.id === params.id);
    if (laudo === undefined) {
      return problema(404, 'nao-encontrado', 'Não encontramos este laudo na sua conta. Confira a lista de resultados.');
    }
    return HttpResponse.json(laudo);
  }),
  http.get('*/v1/links/:token', ({ params }) => {
    if (params.token === 'link-valido') {
      return HttpResponse.json(PORTAL_LAUDO_DOPPLER);
    }
    return problema(410, 'link-expirado', 'Este link de compartilhamento expirou e o laudo não é mais exibido por ele. Peça ao paciente um link novo.');
  }),

  /* --------------------------------------------- Caixa ---------------------------------------- */
  http.get('*/v1/caixas/meu', () => HttpResponse.json(fase1.caixa)),
  http.post('*/v1/caixas', async ({ request }) => {
    const corpo = (await registrar('abrir-caixa', request)) as { fundo_centavos: number };
    fase1.caixa = { ...caixaAberto(), fundo_centavos: corpo.fundo_centavos };
    return HttpResponse.json(fase1.caixa, { status: 201 });
  }),
  http.post('*/v1/caixas/:id/fechamento', async ({ request }) => {
    const corpo = (await registrar('fechamento', request)) as { contado_centavos: number };
    fase1.caixa = null;
    return HttpResponse.json({ esperado_centavos: 74000, contado_centavos: corpo.contado_centavos, diferenca_centavos: corpo.contado_centavos - 74000 });
  }),
  http.post('*/v1/recebimentos', async ({ request }) => {
    const corpo = (await registrar('recebimento', request)) as PedidoRecebimento;
    if (fase1.caixa === null) {
      return problema(409, 'caixa-fechado', 'Não há caixa aberto em seu nome. Nada foi recebido. Abra o caixa e repita o recebimento.');
    }
    const pb = corpo.desconto?.pontos_base ?? 0;
    if (pb > 500 && corpo.desconto?.autorizacao_id === undefined) {
      return problema(
        403,
        'desconto-acima-do-limite',
        'O desconto de 15,00% passa do seu limite de 5,00%. Nada foi recebido. Peça a autorização da gerência ou aplique até 5,00%.',
        { limite_pontos_base: 500 },
      );
    }
    fase1.recebimento = { pb, forma: corpo.forma, consultas: 0 };
    return HttpResponse.json(recebimentoSonia(pb, corpo.forma, false), { status: 201 });
  }),
  http.get('*/v1/recebimentos/:id', () => {
    const r = fase1.recebimento;
    if (r === null) return problema(404, 'nao-encontrado', 'Recebimento não encontrado.');
    r.consultas += 1;
    return HttpResponse.json(recebimentoSonia(r.pb, r.forma, fase1.pspRespondeu));
  }),
  http.post('*/v1/autorizacoes-desconto', async ({ request }) => {
    await registrar('autorizacao', request);
    if (!temPerfil('gerente')) return semPermissao('autorizar desconto');
    const re = reautenticacao('autorizacao');
    if (re !== null) return re;
    return HttpResponse.json({ id: uuid('00000021', 1), codigo: 'AUT-2026-000077', valida_ate: '2026-11-06T13:40:00Z' }, { status: 201 });
  }),
  http.post('*/v1/recebimentos/:id/estornos', async ({ request }) => {
    const corpo = (await registrar('pedir-estorno', request)) as { motivo: string };
    const e: EstornoSimulado = { id: uuid('00000022', 1), situacao: 'pedido', motivo: corpo.motivo, pedido_por: estado.sessao.usuario.id };
    fase1.estornos.set(e.id, e);
    return HttpResponse.json(e, { status: 201 });
  }),
  http.post('*/v1/estornos/:id/aprovacao', async ({ request, params }) => {
    await registrar('aprovar-estorno', request);
    const e: EstornoSimulado = fase1.estornos.get(String(params.id)) ?? {
      id: String(params.id),
      situacao: 'pedido',
      motivo: 'cobrança em duplicidade',
      pedido_por: uuid('0000000a', 7),
    };
    if (e.pedido_por === estado.sessao.usuario.id) {
      return problema(403, 'separacao-de-funcoes', 'Quem pediu o estorno não pode aprovar o próprio pedido. Nada foi estornado. Peça a aprovação à gerência ou ao financeiro.');
    }
    const re = reautenticacao('aprovar-estorno');
    if (re !== null) return re;
    return HttpResponse.json({ id: e.id, situacao: 'aprovado', motivo: e.motivo });
  }),

  /* ------------------------------------------ Contingência ------------------------------------ */
  http.get('*/v1/contingencias/aberta', () => HttpResponse.json(fase1.contingencia)),
  http.post('*/v1/contingencias', async ({ request }) => {
    const corpo = (await registrar('declarar', request)) as PedidoContingencia;
    if (!temPerfil('recepcao', 'supervisor_recepcao', 'coordenacao', 'gerente')) return semPermissao('declarar contingência');
    fase1.contingencia = { ...contingenciaAberta(), nivel: corpo.nivel, motivo: corpo.motivo, declarada_em: corpo.declarada_em };
    return HttpResponse.json(fase1.contingencia, { status: 201 });
  }),
  http.post('*/v1/contingencias/:id/fichas', async ({ request }) => {
    const corpo = (await registrar('ficha', request)) as PedidoFicha;
    const n = corpo.numero;
    if (!(n >= 400 && n <= 499)) {
      return problema(422, 'numero-fora-da-faixa', `A ficha F-ATD ${String(corpo.numero).padStart(4, '0')} não está na faixa reservada para hoje (0400 a 0499). Nada foi lançado. Confira o número no papel.`);
    }
    const ja = fase1.fichas.get(String(corpo.numero));
    if (ja !== undefined) {
      return problema(409, 'ficha-ja-lancada', `A ficha F-ATD ${String(corpo.numero).padStart(4, '0')} já foi lançada no atendimento ${ja.codigo}. Nada foi lançado de novo.`);
    }
    const agendado = itensDaAgenda().find((i) => i.id === corpo.agendamento_id);
    const cadastro = agendado?.paciente;
    if (cadastro !== undefined && cadastro.nascimento !== corpo.nascimento) {
      const codigo = `DIV-2026-${String(31 + fase1.divergenciasAbertas.length).padStart(6, '0')}`;
      fase1.divergenciasAbertas.push(codigo);
      return HttpResponse.json(
        { divergencia: { codigo, tipo: 'identificacao', descricao: `Nascimento do papel difere do cadastro de ${cadastro.nome}. Nenhum cadastro foi alterado; a divergência tem dono e prazo.` } },
        { status: 202 },
      );
    }
    const atendimento: AtendimentoFase1 = {
      id: uuid('0000000f', 700 + n),
      codigo: `ATD-2026-${String(119000 + n).padStart(6, '0')}`,
      accession: `ACC-2026-${String(53000 + n).padStart(6, '0')}`,
      agendamento_id: corpo.agendamento_id,
      paciente: cadastro ?? MARIA_HELENA.paciente,
      sala: agendado?.sala ?? SALA_1,
      requer_contraste: false,
      situacao: 'exame_concluido',
      senha: 'C001',
      pulseira: { codigo: `ETQ-2026-${String(19000 + n).padStart(6, '0')}`, situacao: 'valida', tipo: 'pulseira' },
      criado_em: '2026-11-06T14:10:00Z',
    };
    fase1.fichas.set(String(corpo.numero), atendimento);
    return HttpResponse.json(atendimento, { status: 201 });
  }),
  http.post('*/v1/contingencias/:id/recibos', async ({ request }) => {
    const corpo = (await registrar('recibo', request)) as PedidoRecibo;
    const divergente = corpo.valor_papel_centavos !== 38000;
    if (divergente) fase1.divergenciasAbertas.push('DIV-2026-000040');
    return HttpResponse.json(
      {
        codigo: `REC-2026-00${String(corpo.numero).padStart(4, '0')}`,
        valor_papel_centavos: corpo.valor_papel_centavos,
        valor_recalculado_centavos: 38000,
        divergencia: divergente ? { codigo: 'DIV-2026-000040', tipo: 'preco', descricao: 'Valor do papel difere da tabela vigente na data.' } : null,
      },
      { status: 201 },
    );
  }),
  http.post('*/v1/contingencias/:id/numeros', async ({ request }) => {
    const corpo = (await registrar('numeros', request)) as { de: number; ate: number };
    if (corpo.de > corpo.ate) return problema(422, 'faixa-invalida', 'O número inicial é maior que o final. Nada foi registrado.');
    return HttpResponse.json({ registrados: corpo.ate - corpo.de + 1 });
  }),
  http.post('*/v1/divergencias/:id/resolucao', async ({ request }) => {
    await registrar('resolucao', request);
    if (!temPerfil('gerente', 'coordenacao')) return semPermissao('resolver divergência');
    return HttpResponse.json({ situacao: 'resolvida' });
  }),
  http.post('*/v1/contingencias/:id/encerramento', async ({ request }) => {
    await registrar('encerrar', request);
    if (fase1.divergenciasAbertas.length > 0) {
      return problema(409, 'pendencias', 'A contingência não pode ser encerrada com pendências. Nada foi encerrado. Resolva a lista abaixo.', {
        pendencias: [...fase1.divergenciasAbertas.map((d) => `Divergência ${d} aberta`), 'Ficha F-ATD 0413 sem situação'],
      });
    }
    const encerrada = { ...(fase1.contingencia ?? contingenciaAberta()), situacao: 'encerrada' as const };
    fase1.contingencia = null;
    return HttpResponse.json(encerrada);
  }),
];

export { JULIANA };
