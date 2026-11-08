import type { ProblemaHttp } from './contrato-m0';

/**
 * O servidor respondeu com erro. Quando o corpo é `application/problem+json`, `problema` traz o
 * `detail` (mensagem R8 escrita pelo servidor), o `codigo` estável e o `trace_id` (código de suporte).
 */
export class ErroDaApi extends Error {
  readonly status: number;
  readonly problema: ProblemaHttp;

  constructor(problema: ProblemaHttp) {
    super(problema.detail);
    this.name = 'ErroDaApi';
    this.status = problema.status;
    this.problema = problema;
  }

  get codigo(): string {
    return this.problema.codigo;
  }

  /** Código de suporte, ou cadeia vazia quando o servidor não mandou. */
  get traceId(): string {
    return this.problema.trace_id ?? '';
  }
}

/**
 * Não foi possível falar com o servidor (rede caiu, servidor fora do ar). Numa escrita, não se sabe
 * se o servidor recebeu: reenviar com a mesma `Idempotency-Key` não duplica.
 */
export class ErroSemConexao extends Error {
  constructor(causa?: unknown) {
    super('Não foi possível falar com o servidor.', { cause: causa });
    this.name = 'ErroSemConexao';
  }
}

function ehTexto(valor: unknown): valor is string {
  return typeof valor === 'string';
}

/** Confere se um corpo JSON tem a forma de Problem Details do VidaCor. */
export function ehProblemaHttp(valor: unknown): valor is ProblemaHttp {
  if (typeof valor !== 'object' || valor === null) {
    return false;
  }
  const v = valor as Record<string, unknown>;
  return (
    ehTexto(v.detail) &&
    ehTexto(v.codigo) &&
    (v.trace_id === undefined || v.trace_id === null || ehTexto(v.trace_id)) &&
    typeof v.status === 'number'
  );
}

/**
 * Resposta de erro fora do formato combinado (por exemplo, um proxy no caminho). A tela nunca mente:
 * diz que não houve confirmação, sem inventar o que aconteceu.
 */
export function problemaInesperado(status: number): ProblemaHttp {
  return {
    type: 'about:blank',
    title: 'Resposta inesperada do servidor',
    status,
    detail:
      `O servidor respondeu de um jeito inesperado (HTTP ${String(status)}). ` +
      'Nada foi confirmado por esta tela. Tente de novo; se continuar, avise a TI.',
    codigo: 'resposta-inesperada',
    trace_id: null,
  };
}
