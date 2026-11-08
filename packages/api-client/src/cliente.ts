import type {
  AcessoDeEmergencia,
  AgendaFase1,
  AgendamentoCriado,
  AtendimentoDetalhadoFase1,
  AtendimentoFase1,
  AutorizacaoDeDesconto,
  Caixa,
  Contingencia,
  CorrecaoCadastro,
  Estorno,
  FechamentoDeCaixa,
  FilaDaSala,
  Guia,
  HorarioLivre,
  ItemDaFilaDeLaudos,
  ItemDoPortal,
  Laudo,
  LaudoDoPortal,
  PedidoAcessoEmergencia,
  PedidoAgendamento,
  PedidoAssinatura,
  PedidoAutorizacaoDesconto,
  PedidoBipe,
  PedidoCheckin,
  PedidoConduta,
  PedidoContingencia,
  PedidoConversarAntes,
  PedidoCorrecaoCadastro,
  PedidoEstorno,
  PedidoFicha,
  PedidoGuia,
  PedidoRascunho,
  PedidoRecebimento,
  PedidoRecibo,
  PedidoReimpressao,
  PedidoRetificacao,
  PedidoTriagem,
  Recebimento,
  ReciboLancado,
  Reimpressao,
  ResultadoDaFicha,
  ResultadoDoBipe,
  Triagem,
  Divergencia,
} from './contrato-fase1';
import type { ConsultaAgenda, PacienteResumo, Sala, Saude, Sessao } from './contrato-m0';
import { ErroDaApi, ErroSemConexao, ehProblemaHttp, problemaInesperado } from './erros';

export interface OpcoesDoCliente {
  /** Origem do núcleo. Padrão: a origem da própria página (o Vite encaminha `/v1` ao núcleo). */
  base?: string;
  /** Implementação de `fetch` (testes). */
  fetch?: typeof globalThis.fetch;
  /** Quantas vezes reenviar uma escrita quando a rede falha, sempre com a mesma chave. Padrão: 2. */
  retentativasDeRede?: number;
  /** Espera entre retentativas (testes trocam por espera instantânea). */
  esperar?: (ms: number) => Promise<void>;
  /**
   * Estação de trabalho (`GUICHE-1`, `SALA-1`), gravada no lastro pelo servidor (docs/30 § 0).
   * Só letras, números e hífen, até 32 caracteres; valor fora disso não é enviado.
   */
  estacao?: string;
}

/** Opções de uma escrita que cria algo (G3). */
export interface OpcoesDeEscrita {
  /**
   * UUID gerado UMA vez por operação de negócio (ao abrir o formulário) e reaproveitado em toda
   * retentativa, para que repetir não duplique. Nunca dado pessoal.
   */
  idempotencyKey: string;
}

type Metodo = 'GET' | 'POST' | 'PUT';

interface Pedido {
  metodo: Metodo;
  caminho: string;
  corpo?: unknown;
  escrita?: boolean;
  idempotencyKey?: string;
}

const ESPERA_BASE_MS = 500;
const ESTACAO_VALIDA = /^[A-Za-z0-9-]{1,32}$/;

function seg(valor: string): string {
  return encodeURIComponent(valor);
}

function esperarDeVerdade(ms: number): Promise<void> {
  return new Promise((resolver) => {
    setTimeout(resolver, ms);
  });
}

/** Gera a chave de idempotência de uma operação de negócio. */
export function novaChaveDeIdempotencia(): string {
  return globalThis.crypto.randomUUID();
}

export type ClienteVidaCor = ReturnType<typeof criarCliente>;

/**
 * Cliente fino do núcleo. Sessão em cookie HttpOnly (`credentials: 'include'`); nenhum token passa
 * por aqui nem é guardado no navegador. O CSRF vem de `GET /v1/sessao` e fica só nesta closure.
 */
export function criarCliente(opcoes: OpcoesDoCliente = {}) {
  const base = opcoes.base ?? globalThis.location.origin;
  const buscar = opcoes.fetch ?? globalThis.fetch.bind(globalThis);
  const retentativas = opcoes.retentativasDeRede ?? 2;
  const esperar = opcoes.esperar ?? esperarDeVerdade;
  const estacao = opcoes.estacao !== undefined && ESTACAO_VALIDA.test(opcoes.estacao) ? opcoes.estacao : null;
  let csrf: string | null = null;

  function url(caminho: string): string {
    return new URL(caminho, base).toString();
  }

  async function lerRespostaComSituacao(resposta: Response): Promise<{ status: number; corpo: unknown }> {
    if (resposta.ok) {
      if (resposta.status === 204) {
        return { status: 204, corpo: undefined };
      }
      return { status: resposta.status, corpo: (await resposta.json()) as unknown };
    }
    let corpo: unknown;
    try {
      corpo = await resposta.json();
    } catch {
      corpo = null;
    }
    if (resposta.status === 401) {
      csrf = null;
    }
    throw new ErroDaApi(ehProblemaHttp(corpo) ? corpo : problemaInesperado(resposta.status));
  }

  async function enviarUmaVez(pedido: Pedido, cabecalhos: Record<string, string>): Promise<Response> {
    try {
      return await buscar(url(pedido.caminho), {
        method: pedido.metodo,
        credentials: 'include',
        cache: 'no-store',
        headers: cabecalhos,
        body: pedido.corpo === undefined ? null : JSON.stringify(pedido.corpo),
      });
    } catch (causa) {
      throw new ErroSemConexao(causa);
    }
  }

  async function pedir<T>(pedido: Pedido): Promise<T> {
    const r = await pedirComSituacao(pedido);
    return r.corpo as T;
  }

  async function pedirComSituacao(pedido: Pedido): Promise<{ status: number; corpo: unknown }> {
    const cabecalhos: Record<string, string> = { Accept: 'application/json' };
    if (estacao !== null) {
      cabecalhos['X-Estacao'] = estacao;
    }
    if (pedido.corpo !== undefined) {
      cabecalhos['Content-Type'] = 'application/json';
    }
    if (pedido.escrita === true) {
      if (csrf === null) {
        await obterSessao();
      }
      if (csrf !== null) {
        cabecalhos['X-CSRF-Token'] = csrf;
      }
    }
    if (pedido.idempotencyKey !== undefined) {
      cabecalhos['Idempotency-Key'] = pedido.idempotencyKey;
    }
    // Só escrita com chave de idempotência é reenviada sozinha: a chave garante que não duplica.
    const maximo = pedido.idempotencyKey === undefined ? 0 : retentativas;
    for (let tentativa = 0; ; tentativa += 1) {
      try {
        const resposta = await enviarUmaVez(pedido, cabecalhos);
        return await lerRespostaComSituacao(resposta);
      } catch (erro) {
        if (erro instanceof ErroSemConexao && tentativa < maximo) {
          await esperar(ESPERA_BASE_MS * 2 ** tentativa);
          continue;
        }
        throw erro;
      }
    }
  }

  async function obterSessao(): Promise<Sessao> {
    const sessao = await pedir<Sessao>({ metodo: 'GET', caminho: '/v1/sessao' });
    csrf = sessao.csrf;
    return sessao;
  }

  /** Escrita que cria registro: CSRF e `Idempotency-Key` (G3). */
  function criar<T>(metodo: Metodo, caminho: string, corpo: unknown, opcoesDeEscrita: OpcoesDeEscrita): Promise<T> {
    return pedir<T>({ metodo, caminho, corpo, escrita: true, idempotencyKey: opcoesDeEscrita.idempotencyKey });
  }

  /** Escrita sem chave (atualização idempotente por natureza, como salvar rascunho). */
  function escrever<T>(metodo: Metodo, caminho: string, corpo: unknown): Promise<T> {
    return pedir<T>({ metodo, caminho, corpo, escrita: true });
  }

  function ler<T>(caminho: string): Promise<T> {
    return pedir<T>({ metodo: 'GET', caminho });
  }

  return {
    saude: (): Promise<Saude> => ler<Saude>('/v1/saude'),

    sessao: {
      obter: obterSessao,
      /** Endereço que inicia a entrada (OIDC no servidor). A página navega para ele. */
      urlDeEntrada: (): string => url('/v1/sessao/entrar'),
      /**
       * Reautenticação de ato sensível (SEC-AUT-5, docs/30 § 0). `voltar` é um caminho da própria
       * aplicação para onde o servidor devolve; outra origem nunca é aceita (vira `/`).
       */
      urlDeReautenticacao: (voltar: string): string => {
        const destino = voltar.startsWith('/') && !voltar.startsWith('//') ? voltar : '/';
        return url(`/v1/sessao/reautenticar?${new URLSearchParams({ voltar: destino }).toString()}`);
      },
      sair: async (): Promise<void> => {
        await pedir<undefined>({ metodo: 'POST', caminho: '/v1/sessao/sair', escrita: true });
        csrf = null;
      },
    },

    salas: {
      listar: (): Promise<Sala[]> => ler<Sala[]>('/v1/salas'),
      /** Fila do dia da sala (ato `sala.ver`). */
      fila: (salaId: string): Promise<FilaDaSala> => ler<FilaDaSala>(`/v1/salas/${seg(salaId)}/fila`),
      /** Bipe da pulseira (leitor = teclado + Enter). Quem decide se libera é o servidor. */
      bipar: (salaId: string, corpo: PedidoBipe, o: OpcoesDeEscrita): Promise<ResultadoDoBipe> =>
        criar<ResultadoDoBipe>('POST', `/v1/salas/${seg(salaId)}/bipes`, corpo, o),
      horariosLivres: (salaId: string, data: string): Promise<HorarioLivre[]> =>
        ler<HorarioLivre[]>(`/v1/salas/${seg(salaId)}/horarios-livres?${new URLSearchParams({ data }).toString()}`),
    },

    agenda: {
      obter: (consulta: ConsultaAgenda): Promise<AgendaFase1> => {
        const parametros = new URLSearchParams({ data: consulta.data });
        if (consulta.sala_id !== undefined && consulta.sala_id !== '') {
          parametros.set('sala_id', consulta.sala_id);
        }
        return ler<AgendaFase1>(`/v1/agenda?${parametros.toString()}`);
      },
    },

    atendimentos: {
      /** Check-in. O servidor confere os dois identificadores e gera senha, pulseira e códigos (G1). */
      criar: (corpo: PedidoCheckin, o: OpcoesDeEscrita): Promise<AtendimentoFase1> =>
        criar<AtendimentoFase1>('POST', '/v1/atendimentos', corpo, o),
      obter: (id: string): Promise<AtendimentoDetalhadoFase1> =>
        ler<AtendimentoDetalhadoFase1>(`/v1/atendimentos/${seg(id)}`),
      /** Triagem com contraste. O servidor calcula a TFG e o resultado (FC-TRI-07). */
      registrarTriagem: (id: string, corpo: PedidoTriagem, o: OpcoesDeEscrita): Promise<Triagem> =>
        criar<Triagem>('POST', `/v1/atendimentos/${seg(id)}/triagem`, corpo, o),
      /** Conduta médica (reautenticação). */
      registrarConduta: (id: string, corpo: PedidoConduta, o: OpcoesDeEscrita): Promise<Triagem> =>
        criar<Triagem>('POST', `/v1/atendimentos/${seg(id)}/triagem/conduta`, corpo, o),
    },

    etiquetas: {
      reimprimir: (codigo: string, corpo: PedidoReimpressao, o: OpcoesDeEscrita): Promise<Reimpressao> =>
        criar<Reimpressao>('POST', `/v1/etiquetas/${seg(codigo)}/reimpressao`, corpo, o),
    },

    pacientes: {
      /** Até 20 pacientes com nome e nascimento. O texto vai na consulta à API, nunca no endereço da página. */
      buscar: (texto: string): Promise<PacienteResumo[]> =>
        ler<PacienteResumo[]>(`/v1/pacientes?${new URLSearchParams({ busca: texto }).toString()}`),
      /** Correção de cadastro com motivo (reautenticação). */
      corrigir: (id: string, corpo: PedidoCorrecaoCadastro, o: OpcoesDeEscrita): Promise<CorrecaoCadastro> =>
        criar<CorrecaoCadastro>('POST', `/v1/pacientes/${seg(id)}/correcao`, corpo, o),
    },

    guias: {
      registrar: (corpo: PedidoGuia, o: OpcoesDeEscrita): Promise<Guia> => criar<Guia>('POST', '/v1/guias', corpo, o),
    },

    agendamentos: {
      criar: (corpo: PedidoAgendamento, o: OpcoesDeEscrita): Promise<AgendamentoCriado> =>
        criar<AgendamentoCriado>('POST', '/v1/agendamentos', corpo, o),
      /** Confirma a reserva provisória com a guia entregue (FC-AGD-09). */
      vincularGuia: (id: string, guiaId: string, o: OpcoesDeEscrita): Promise<AgendamentoCriado> =>
        criar<AgendamentoCriado>('POST', `/v1/agendamentos/${seg(id)}/guia`, { guia_id: guiaId }, o),
    },

    laudos: {
      fila: (): Promise<ItemDaFilaDeLaudos[]> => ler<ItemDaFilaDeLaudos[]>('/v1/laudos/fila'),
      obter: (id: string): Promise<Laudo> => ler<Laudo>(`/v1/laudos/${seg(id)}`),
      /** Salvamento automático do rascunho (R6). Repetir grava o mesmo texto. */
      salvarRascunho: (id: string, corpo: PedidoRascunho): Promise<Laudo> =>
        escrever<Laudo>('PUT', `/v1/laudos/${seg(id)}/rascunho`, corpo),
      assinar: (id: string, corpo: PedidoAssinatura, o: OpcoesDeEscrita): Promise<Laudo> =>
        criar<Laudo>('POST', `/v1/laudos/${seg(id)}/assinatura`, corpo, o),
      retificar: (id: string, corpo: PedidoRetificacao, o: OpcoesDeEscrita): Promise<Laudo> =>
        criar<Laudo>('POST', `/v1/laudos/${seg(id)}/retificacao`, corpo, o),
      conversarAntes: (id: string, corpo: PedidoConversarAntes): Promise<Laudo> =>
        escrever<Laudo>('POST', `/v1/laudos/${seg(id)}/conversar-antes`, corpo),
      liberar: (id: string, o: OpcoesDeEscrita): Promise<Laudo> =>
        criar<Laudo>('POST', `/v1/laudos/${seg(id)}/liberacao`, {}, o),
      acessoDeEmergencia: (id: string, corpo: PedidoAcessoEmergencia, o: OpcoesDeEscrita): Promise<AcessoDeEmergencia> =>
        criar<AcessoDeEmergencia>('POST', `/v1/laudos/${seg(id)}/acesso-emergencia`, corpo, o),
    },

    portal: {
      laudos: (): Promise<ItemDoPortal[]> => ler<ItemDoPortal[]>('/v1/portal/laudos'),
      laudo: (id: string): Promise<LaudoDoPortal> => ler<LaudoDoPortal>(`/v1/portal/laudos/${seg(id)}`),
      /** Laudo compartilhado por link; `410 link-expirado` quando venceu. */
      link: (token: string): Promise<LaudoDoPortal> => ler<LaudoDoPortal>(`/v1/links/${seg(token)}`),
    },

    caixas: {
      meu: (): Promise<Caixa | null> => ler<Caixa | null>('/v1/caixas/meu'),
      abrir: (fundoCentavos: number, o: OpcoesDeEscrita): Promise<Caixa> =>
        criar<Caixa>('POST', '/v1/caixas', { fundo_centavos: fundoCentavos }, o),
      fechar: (id: string, contadoCentavos: number, o: OpcoesDeEscrita): Promise<FechamentoDeCaixa> =>
        criar<FechamentoDeCaixa>('POST', `/v1/caixas/${seg(id)}/fechamento`, { contado_centavos: contadoCentavos }, o),
    },

    recebimentos: {
      /** O servidor calcula o total pela tabela vigente; a tela não manda total (FC-CXA-01, 02). */
      criar: (corpo: PedidoRecebimento, o: OpcoesDeEscrita): Promise<Recebimento> =>
        criar<Recebimento>('POST', '/v1/recebimentos', corpo, o),
      pedirEstorno: (id: string, corpo: PedidoEstorno, o: OpcoesDeEscrita): Promise<Estorno> =>
        criar<Estorno>('POST', `/v1/recebimentos/${seg(id)}/estornos`, corpo, o),
    },

    estornos: {
      /** Aprovação (reautenticação; quem pediu não aprova: `403 separacao-de-funcoes`). */
      aprovar: (id: string, o: OpcoesDeEscrita): Promise<Estorno> =>
        criar<Estorno>('POST', `/v1/estornos/${seg(id)}/aprovacao`, {}, o),
    },

    autorizacoesDeDesconto: {
      criar: (corpo: PedidoAutorizacaoDesconto, o: OpcoesDeEscrita): Promise<AutorizacaoDeDesconto> =>
        criar<AutorizacaoDeDesconto>('POST', '/v1/autorizacoes-desconto', corpo, o),
    },

    contingencias: {
      /** A `chave` vai no corpo (gerada na estação, docs/30 § 6) e também como `Idempotency-Key`. */
      declarar: (corpo: PedidoContingencia): Promise<Contingencia> =>
        criar<Contingencia>('POST', '/v1/contingencias', corpo, { idempotencyKey: corpo.chave }),
      aberta: (): Promise<Contingencia | null> => ler<Contingencia | null>('/v1/contingencias/aberta'),
      /** 201 com o atendimento ou 202 com a divergência aberta (nascimento do papel difere do cadastro). */
      lancarFicha: async (id: string, corpo: PedidoFicha, o: OpcoesDeEscrita): Promise<ResultadoDaFicha> => {
        const r = await pedirComSituacao({
          metodo: 'POST',
          caminho: `/v1/contingencias/${seg(id)}/fichas`,
          corpo,
          escrita: true,
          idempotencyKey: o.idempotencyKey,
        });
        if (r.status === 202) {
          return { situacao: 'divergencia-aberta', divergencia: (r.corpo as { divergencia: Divergencia }).divergencia };
        }
        return { situacao: 'lancada', atendimento: r.corpo as AtendimentoFase1 };
      },
      lancarRecibo: (id: string, corpo: PedidoRecibo, o: OpcoesDeEscrita): Promise<ReciboLancado> =>
        criar<ReciboLancado>('POST', `/v1/contingencias/${seg(id)}/recibos`, corpo, o),
      encerrar: (id: string, o: OpcoesDeEscrita): Promise<Contingencia> =>
        criar<Contingencia>('POST', `/v1/contingencias/${seg(id)}/encerramento`, {}, o),
    },

    /** Endereço do fluxo SSE (`abrirEventos`). */
    urlDeEventos: (): string => url('/v1/eventos'),
  };
}
