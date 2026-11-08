import type { EventoEstudoRecebido } from './contrato-m0';

/** O mínimo de `EventSource` que usamos (permite trocar por um falso nos testes). */
export interface FonteDeEventos {
  readonly readyState: number;
  onopen: ((ev: Event) => unknown) | null;
  onerror: ((ev: Event) => unknown) | null;
  addEventListener(tipo: string, ouvinte: (ev: MessageEvent<string>) => void): void;
  close(): void;
}

export type EstadoDosEventos = 'conectando' | 'conectado' | 'reconectando';

export interface OpcoesDosEventos {
  url: string;
  aoEstudoRecebido: (evento: EventoEstudoRecebido) => void;
  aoMudarEstado?: (estado: EstadoDosEventos) => void;
  /** Evento fora do formato combinado: descartado (enum ou campo desconhecido não é aceito). */
  aoDescartar?: (tipo: string) => void;
  criarFonte?: (url: string) => FonteDeEventos;
  agendar?: (acao: () => void, ms: number) => () => void;
  esperaInicialMs?: number;
  esperaMaximaMs?: number;
}

const FECHADA = 2;

function agendarDeVerdade(acao: () => void, ms: number): () => void {
  const id = setTimeout(acao, ms);
  return () => {
    clearTimeout(id);
  };
}

/** Lê e confere o evento `estudo-recebido`. Devolve `null` se o formato não bate. */
export function lerEstudoRecebido(dados: string): EventoEstudoRecebido | null {
  let valor: unknown;
  try {
    valor = JSON.parse(dados);
  } catch {
    return null;
  }
  if (typeof valor !== 'object' || valor === null) {
    return null;
  }
  const v = valor as Record<string, unknown>;
  const atendimentoId = v.atendimento_id;
  const accession = v.accession;
  const imagens = v.imagens;
  if (
    typeof atendimentoId !== 'string' ||
    typeof accession !== 'string' ||
    typeof imagens !== 'number' ||
    !Number.isInteger(imagens) ||
    imagens < 0
  ) {
    return null;
  }
  return { atendimento_id: atendimentoId, accession, imagens };
}

/**
 * Abre `GET /v1/eventos` (SSE, mesma origem, cookie de sessão). O navegador já reconecta sozinho
 * quando a conexão cai; quando ele desiste (fonte fechada, por exemplo depois de um 5xx), reabrimos
 * com espera crescente até `esperaMaximaMs`.
 */
export function abrirEventos(opcoes: OpcoesDosEventos): { fechar: () => void } {
  const criarFonte = opcoes.criarFonte ?? ((u: string): FonteDeEventos => new EventSource(u));
  const agendar = opcoes.agendar ?? agendarDeVerdade;
  const esperaInicial = opcoes.esperaInicialMs ?? 1000;
  const esperaMaxima = opcoes.esperaMaximaMs ?? 30000;
  let espera = esperaInicial;
  let fonte: FonteDeEventos | null = null;
  let cancelarReabertura: (() => void) | null = null;
  let fechado = false;

  function mudar(estado: EstadoDosEventos): void {
    opcoes.aoMudarEstado?.(estado);
  }

  function abrir(): void {
    if (fechado) {
      return;
    }
    const atual = criarFonte(opcoes.url);
    fonte = atual;
    atual.onopen = () => {
      espera = esperaInicial;
      mudar('conectado');
    };
    atual.onerror = () => {
      mudar('reconectando');
      if (atual.readyState === FECHADA && !fechado) {
        atual.close();
        cancelarReabertura = agendar(abrir, espera);
        espera = Math.min(espera * 2, esperaMaxima);
      }
    };
    atual.addEventListener('estudo-recebido', (ev) => {
      const evento = lerEstudoRecebido(ev.data);
      if (evento === null) {
        opcoes.aoDescartar?.('estudo-recebido');
        return;
      }
      opcoes.aoEstudoRecebido(evento);
    });
  }

  mudar('conectando');
  abrir();

  return {
    fechar: () => {
      fechado = true;
      cancelarReabertura?.();
      fonte?.close();
    },
  };
}
