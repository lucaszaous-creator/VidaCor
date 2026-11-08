import { HttpResponse } from 'msw';

/** Resposta `application/problem+json` como o núcleo manda (docs/30 § 0), com campos extras. */
export function problema(status: number, codigo: string, detail: string, extras: Record<string, unknown> = {}) {
  return HttpResponse.json(
    { type: `https://vidacor.local/erros/${codigo}`, title: codigo, status, detail, codigo, trace_id: `trace-${codigo}`, ...extras },
    { status, headers: { 'Content-Type': 'application/problem+json' } },
  );
}
