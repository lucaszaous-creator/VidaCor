import { useRouter, useRouterState } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { useDependencias } from './contexto';

/**
 * Reautenticação de ato sensível (SEC-AUT-5, docs/30 § 0). Quando o servidor responde
 * `401 reautenticacao-necessaria`, a tela leva a `GET /v1/sessao/reautenticar?voltar=<rota atual>`.
 * Na volta, a rota traz `retomar` (qual ato) e `chave` (a mesma `Idempotency-Key`), e a tela repete o
 * ato sem duplicar. Na URL vão só esses dois e identificadores técnicos: nunca nome, nascimento,
 * texto clínico, PIN ou motivo.
 *
 * Caminho preferido (DEC-2026-11-08-54): a confirmação abre numa JANELA à parte que volta para
 * `/reautenticado`; essa página avisa a tela de origem pelo `BroadcastChannel` (mesma origem, sem
 * dado nenhum na mensagem) e a tela repete o ato com a mesma chave, sem perder o que foi digitado.
 * Se o navegador bloquear a janela, cai no caminho por redirecionamento acima.
 */

/** Canal da mesma origem usado só para dizer "identidade confirmada". */
export const CANAL_DE_REAUTENTICACAO = 'vidacor-reautenticacao';
export const MENSAGEM_REAUTENTICADO = 'reautenticado';

export type ResultadoDaJanela = 'confirmada' | 'bloqueada';

/**
 * Abre a confirmação de identidade numa janela e espera o aviso de `/reautenticado`. Devolve
 * `bloqueada` na hora quando o navegador não abre a janela. Se a pessoa fechar a janela sem
 * confirmar, a promessa não se resolve e o erro continua na tela, com o botão para tentar de novo.
 */
export function abrirJanelaDeReautenticacao(url: string, janela: Window = window): Promise<ResultadoDaJanela> {
  const aberta = janela.open(url, 'vidacor-reautenticacao', 'popup,width=520,height=720') as Window | null | undefined;
  if (aberta === null || aberta === undefined) {
    return Promise.resolve('bloqueada');
  }
  return new Promise((resolver) => {
    const canal = new BroadcastChannel(CANAL_DE_REAUTENTICACAO);
    canal.onmessage = (e: MessageEvent<unknown>) => {
      if (typeof e.data === 'object' && e.data !== null && (e.data as { tipo?: unknown }).tipo === MENSAGEM_REAUTENTICADO) {
        canal.close();
        resolver('confirmada');
      }
    };
  });
}

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

/**
 * Devolve a função que pede a reautenticação. Com `repetir`, abre a janela e, confirmada a
 * identidade, chama `repetir` (que reenvia com a mesma chave). Sem janela, sai da tela e volta para a
 * rota atual com `retomar` e `chave`.
 */
export function useReautenticar() {
  const { api, irPara } = useDependencias();
  const local = useRouterState({ select: (s) => ({ caminho: s.location.pathname, busca: s.location.search }) });
  const porRedirecionamento = (ato: string, chave: string, extras: Record<string, string>): void => {
    const busca = new URLSearchParams({ ...paraTexto(local.busca), ...extras, retomar: ato, chave });
    irPara(api.sessao.urlDeReautenticacao(`${local.caminho}?${busca.toString()}`));
  };
  return (ato: string, chave: string, extras: Record<string, string> = {}, repetir?: () => void): void => {
    if (repetir === undefined || typeof BroadcastChannel === 'undefined') {
      porRedirecionamento(ato, chave, extras);
      return;
    }
    void abrirJanelaDeReautenticacao(api.sessao.urlDeReautenticacao('/reautenticado')).then((r) => {
      if (r === 'confirmada') {
        repetir();
      } else {
        porRedirecionamento(ato, chave, extras);
      }
    });
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
