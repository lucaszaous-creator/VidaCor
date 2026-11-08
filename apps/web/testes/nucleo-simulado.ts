/**
 * Núcleo simulado com MSW para os testes de TELA. Ele só imita as respostas do contrato v1
 * (docs/29 § 6.2) para conferir o que a tela mostra; a prova das regras (dois identificadores,
 * idempotência, concorrência) é dos testes de integração do núcleo, com banco real (G4).
 */
import type { AtendimentoFase1, ItemDaAgendaFase1, PedidoCheckin, Sessao } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';
import { agendaDoDia, atendimentoDe, itensDaAgenda, SALAS, SESSAO } from './dados/ficticios';
import { manipuladoresFase1, reiniciarFase1 } from './nucleo-fase1';
import { comunicacaoSimulada, lastroSimulado, manipuladoresComunicacao, manipuladoresEstoque, manipuladoresIndicadores, manipuladoresLastro } from './nucleo-estoque';
import { manipuladoresLaudoEstruturado, reiniciarLaudoEstruturado } from './nucleo-laudo-estruturado';
import { manipuladoresMenu } from './nucleo-menu';
import { manipuladoresCadastro, manipuladoresPortal, portalSimulado } from './nucleo-portal';
import { manipuladoresAReceber, manipuladoresPendencias, reiniciarPendencias } from './nucleo-pendencias';
import { problema } from './problema';

export { problema } from './problema';

export interface EstadoDoNucleo {
  /** Quem está logado (troca por teste para provar o que cada perfil vê). */
  sessao: Sessao;
  itens: ItemDaAgendaFase1[];
  porChave: Map<string, AtendimentoFase1>;
  pedidosDeCheckin: { chave: string | null; csrf: string | null; corpo: PedidoCheckin }[];
}

export const estado: EstadoDoNucleo = { sessao: SESSAO, itens: itensDaAgenda(), porChave: new Map(), pedidosDeCheckin: [] };

export function reiniciarNucleo(): void {
  estado.sessao = SESSAO;
  estado.itens = itensDaAgenda();
  estado.porChave.clear();
  estado.pedidosDeCheckin = [];
  reiniciarFase1();
  reiniciarPendencias();
  reiniciarLaudoEstruturado();
  portalSimulado.logado = false;
  lastroSimulado.integra = true;
  comunicacaoSimulada.whatsapp = true;
}

function normalizar(nome: string): string {
  return nome.normalize('NFC').trim().replace(/\s+/g, ' ').toLocaleLowerCase('pt-BR');
}

export const manipuladores = [
  http.get('*/v1/saude', () => HttpResponse.json({ situacao: 'ok' })),
  http.get('*/v1/sessao', () => HttpResponse.json(estado.sessao)),
  http.post('*/v1/sessao/sair', () => new HttpResponse(null, { status: 204 })),
  http.get('*/v1/salas', () => HttpResponse.json(SALAS)),
  http.get('*/v1/agenda', ({ request }) => {
    const url = new URL(request.url);
    const data = url.searchParams.get('data') ?? '';
    const sala = url.searchParams.get('sala_id');
    const base = agendaDoDia(data, sala);
    const ids = new Set(base.itens.map((i) => i.id));
    return HttpResponse.json({ data, itens: estado.itens.filter((i) => ids.has(i.id)) });
  }),
  http.get('*/v1/atendimentos/:id', ({ params }) => {
    const item = estado.itens.find((i) => i.atendimento_id === params.id);
    if (item === undefined) {
      return problema(404, 'atendimento-nao-encontrado', 'Atendimento não encontrado.');
    }
    const criado = [...estado.porChave.values()].find((a) => a.id === params.id);
    return HttpResponse.json(criado === undefined ? atendimentoDe(item, 1) : { ...criado, estudo: null, triagem: null, laudo_situacao: null });
  }),
  http.post('*/v1/atendimentos', async ({ request }) => {
    const corpo = (await request.json()) as PedidoCheckin;
    const chave = request.headers.get('Idempotency-Key');
    const csrf = request.headers.get('X-CSRF-Token');
    estado.pedidosDeCheckin.push({ chave, csrf, corpo });
    if (csrf !== estado.sessao.csrf) {
      return problema(403, 'csrf-invalido', 'Pedido sem a confirmação de segurança. Recarregue a página.');
    }
    if (chave === null) {
      return problema(400, 'idempotency-key-ausente', 'Pedido sem chave de idempotência.');
    }
    const repetido = estado.porChave.get(chave);
    if (repetido !== undefined) {
      return HttpResponse.json(repetido, { status: 201 });
    }
    if (!estado.sessao.usuario.perfis.some((p) => p === 'recepcao' || p === 'supervisor_recepcao')) {
      return problema(403, 'sem-permissao', 'Seu perfil não faz check-in. Nada foi gravado. Peça a uma pessoa da recepção.');
    }
    if (!corpo.confirmacao.documento_com_foto) {
      return problema(422, 'identificacao-incompleta', 'Falta conferir o documento com foto. Nada foi gravado: nenhuma senha ou pulseira foi emitida. Confira o documento e repita.');
    }
    const item = estado.itens.find((i) => i.id === corpo.agendamento_id);
    if (item === undefined) {
      return problema(404, 'agendamento-nao-encontrado', 'O agendamento não foi encontrado. Nada foi gravado. Atualize a agenda.');
    }
    if (item.atendimento_id !== null && item.atendimento_id !== undefined) {
      return problema(409, 'checkin-ja-realizado', 'O check-in deste agendamento já foi feito. Nada foi gravado de novo. Abra o atendimento na agenda.');
    }
    if (normalizar(corpo.confirmacao.nome) !== normalizar(item.paciente.nome) || corpo.confirmacao.nascimento !== item.paciente.nascimento) {
      return problema(
        422,
        'identificacao-nao-confere',
        'O nome ou o nascimento informado não confere com o cadastro. Nada foi gravado. Confira o documento do paciente e digite de novo.',
      );
    }
    const { id, codigo, accession, agendamento_id, paciente, sala, requer_contraste, situacao, senha, pulseira, criado_em } = atendimentoDe(
      item,
      estado.porChave.size + 1,
    );
    const criado: AtendimentoFase1 = { id, codigo, accession, agendamento_id, paciente, sala, requer_contraste, situacao, senha, pulseira, criado_em };
    estado.porChave.set(chave, criado);
    item.atendimento_id = criado.id;
    item.situacao = 'em_atendimento';
    return HttpResponse.json(criado, { status: 201 });
  }),
];

export const servidor = setupServer(...manipuladoresComunicacao, ...manipuladoresIndicadores, ...manipuladoresLaudoEstruturado, ...manipuladoresLastro, ...manipuladoresEstoque, ...manipuladoresCadastro, ...manipuladoresPortal, ...manipuladoresMenu, ...manipuladoresAReceber, ...manipuladoresPendencias, ...manipuladoresFase1, ...manipuladores);
