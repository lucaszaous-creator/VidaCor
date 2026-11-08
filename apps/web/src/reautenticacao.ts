import { useRouter, useRouterState } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { useDependencias } from './contexto';

/**
 * Reautenticação de ato sensível (SEC-AUT-5, docs/30 § 0). Quando o servidor responde
 * `401 reautenticacao-necessaria`, a tela leva a `GET /v1/sessao/reautenticar?voltar=<rota atual>`.
 * Na volta, a rota traz `retomar` (qual ato) e `chave` (a mesma `Idempotency-Key`), e a tela repete o
 * ato sem duplicar. Na URL vão só esses dois e identificadores técnicos: nunca nome, nascimento,
 * texto clínico, PIN ou motivo. O texto do laudo é preservado no rascunho salvo no servidor; o resto
 * que a pessoa digitou volta a ser pedido, e a tela diz isso antes de sair.
 */

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const ATO = /^[a-z][a-z-]{0,39}$/;

export interface Retomada {
  retomar: string;
  chave: string;
}

/** Lê `retomar` e `chave` da busca da rota, só no formato esperado. */
export function lerRetomada(busca: Record<string, unknown>): Retomada | null {
  const { retomar, chave } = busca;
  if (typeof retomar === 'string' && ATO.test(retomar) && typeof chave === 'string' && UUID.test(chave)) {
    return { retomar, chave };
  }
  return null;
}

function paraTexto(busca: Record<string, unknown>): Record<string, string> {
  const r: Record<string, string> = {};
  for (const [k, v] of Object.entries(busca)) {
    if (typeof v === 'string') {
      r[k] = v;
    } else if (typeof v === 'number' && Number.isFinite(v)) {
      r[k] = String(v);
    }
  }
  return r;
}

/** Devolve a função que leva à reautenticação, com a volta para a rota atual. */
export function useReautenticar() {
  const { api, irPara } = useDependencias();
  const local = useRouterState({ select: (s) => ({ caminho: s.location.pathname, busca: s.location.search }) });
  return (ato: string, chave: string, extras: Record<string, string> = {}): void => {
    const busca = new URLSearchParams({ ...paraTexto(local.busca), ...extras, retomar: ato, chave });
    irPara(api.sessao.urlDeReautenticacao(`${local.caminho}?${busca.toString()}`));
  };
}

/**
 * Lê a retomada UMA vez ao abrir a tela e tira `retomar` e `chave` do endereço, para que recarregar
 * a página não repita o ato.
 */
export function useRetomada(): Retomada | null {
  const router = useRouter();
  const local = useRouterState({ select: (s) => ({ caminho: s.location.pathname, busca: s.location.search }) });
  const [retomada] = useState(() => lerRetomada(local.busca));

  useEffect(() => {
    if (retomada === null) {
      return;
    }
    const resto = paraTexto(local.busca);
    delete resto.retomar;
    delete resto.chave;
    const busca = new URLSearchParams(resto).toString();
    router.history.replace(busca === '' ? local.caminho : `${local.caminho}?${busca}`);
    // Só na abertura da tela.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return retomada;
}
