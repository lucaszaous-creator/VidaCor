import type {
  Agenda,
  Atendimento,
  AtendimentoDetalhado,
  ConsultaAgenda,
  NovoAtendimento,
  Sala,
  Saude,
  Sessao,
} from './contrato-m0';
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
}

/** Opções de uma escrita que cria algo (G3). */
export interface OpcoesDeEscrita {
  /**
   * UUID gerado UMA vez por operação de negócio (ao abrir o formulário) e reaproveitado em toda
   * retentativa, para que repetir não duplique. Nunca dado pessoal.
   */
  idempotencyKey: string;
}

type Metodo = 'GET' | 'POST';

interface Pedido {
  metodo: Metodo;
  caminho: string;
  corpo?: unknown;
  escrita?: boolean;
  idempotencyKey?: string;
}

const ESPERA_BASE_MS = 500;

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
  let csrf: string | null = null;

  function url(caminho: string): string {
    return new URL(caminho, base).toString();
  }

  async function lerResposta<T>(resposta: Response): Promise<T> {
    if (resposta.ok) {
      if (resposta.status === 204) {
        return undefined as T;
      }
      return (await resposta.json()) as T;
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
    const cabecalhos: Record<string, string> = { Accept: 'application/json' };
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
        return await lerResposta<T>(resposta);
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

  return {
    saude: (): Promise<Saude> => pedir<Saude>({ metodo: 'GET', caminho: '/v1/saude' }),

    sessao: {
      obter: obterSessao,
      /** Endereço que inicia a entrada (OIDC no servidor). A página navega para ele. */
      urlDeEntrada: (): string => url('/v1/sessao/entrar'),
      sair: async (): Promise<void> => {
        await pedir<undefined>({ metodo: 'POST', caminho: '/v1/sessao/sair', escrita: true });
        csrf = null;
      },
    },

    salas: {
      listar: (): Promise<Sala[]> => pedir<Sala[]>({ metodo: 'GET', caminho: '/v1/salas' }),
    },

    agenda: {
      obter: (consulta: ConsultaAgenda): Promise<Agenda> => {
        const parametros = new URLSearchParams({ data: consulta.data });
        if (consulta.sala_id !== undefined && consulta.sala_id !== '') {
          parametros.set('sala_id', consulta.sala_id);
        }
        return pedir<Agenda>({ metodo: 'GET', caminho: `/v1/agenda?${parametros.toString()}` });
      },
    },

    atendimentos: {
      /** Check-in. O servidor confere os dois identificadores e gera os códigos (G1). */
      criar: (corpo: NovoAtendimento, opcoesDeEscrita: OpcoesDeEscrita): Promise<Atendimento> =>
        pedir<Atendimento>({
          metodo: 'POST',
          caminho: '/v1/atendimentos',
          corpo,
          escrita: true,
          idempotencyKey: opcoesDeEscrita.idempotencyKey,
        }),
      obter: (id: string): Promise<AtendimentoDetalhado> =>
        pedir<AtendimentoDetalhado>({
          metodo: 'GET',
          caminho: `/v1/atendimentos/${encodeURIComponent(id)}`,
        }),
    },

    /** Endereço do fluxo SSE (`abrirEventos`). */
    urlDeEventos: (): string => url('/v1/eventos'),
  };
}
