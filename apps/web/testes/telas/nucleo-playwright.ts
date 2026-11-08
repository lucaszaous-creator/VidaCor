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
    if (c === '/v1/lastro/cadeia') {
      const evs = Array.from({ length: 30 }, (_, i) => ({ sequencia: i + 1, dominio: 'recepcao', tipo: 'evento_de_teste', registro_codigo: 'ATD-2026-000031', criado_em: '2026-11-06T11:00:00Z', origem: 'recepcao', hash: '3f9ac21b77e0', integro: true }));
      return json(route, { paciente: { id: 'p', nome: 'Sônia Maria Castro', nascimento: '1962-02-03' }, atendimento_codigo: 'ATD-2026-000031', nos: Array.from({ length: 12 }, (_, i) => ({ tipo: 'etiqueta', id: `n${String(i)}`, codigo: `ETQ-2026-0000${String(i)}`, situacao: 'valida', criado_em: '2026-11-06T11:00:00Z' })), eventos: evs, integra: true });
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
