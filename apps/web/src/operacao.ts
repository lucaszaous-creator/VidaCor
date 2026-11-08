import { useMutation } from '@tanstack/react-query';
import { ErroDaApi, ehReautenticacao, novaChaveDeIdempotencia } from '@vidacor/api-client';
import { useRef } from 'react';

export interface OpcoesDaOperacao<R> {
  /** Chave herdada (volta da reautenticação): o ato se repete com a mesma `Idempotency-Key`. */
  chaveInicial?: string | undefined;
  aoConcluir?: (resultado: R) => void | Promise<void>;
  aoFalhar?: (erro: unknown) => void | Promise<void>;
}

/**
 * Uma operação de negócio que grava algo no servidor (G3). A `Idempotency-Key` nasce uma vez e é
 * reaproveitada em toda repetição: queda de rede (o cliente já reenvia sozinho) e volta da
 * reautenticação. Só muda depois que o servidor RECUSOU (4xx que não seja reautenticação), porque o
 * próximo envio é outra tentativa, com outros dados; ou quando a tela começa outra operação.
 */
export function useOperacao<V, R>(fn: (variaveis: V, chave: string) => Promise<R>, opcoes: OpcoesDaOperacao<R> = {}) {
  const chave = useRef<string>(opcoes.chaveInicial ?? novaChaveDeIdempotencia());
  const recusada = useRef(false);

  const mutacao = useMutation<R, unknown, V>({
    mutationFn: (variaveis) => {
      if (recusada.current) {
        chave.current = novaChaveDeIdempotencia();
        recusada.current = false;
      }
      return fn(variaveis, chave.current);
    },
    onSuccess: async (resultado) => {
      await opcoes.aoConcluir?.(resultado);
    },
    onError: async (erro) => {
      if (erro instanceof ErroDaApi && erro.status < 500 && !ehReautenticacao(erro)) {
        recusada.current = true;
      }
      await opcoes.aoFalhar?.(erro);
    },
  });

  return {
    enviar: (variaveis: V) => {
      mutacao.mutate(variaveis);
    },
    enviando: mutacao.isPending,
    erro: mutacao.isError ? mutacao.error : null,
    resultado: mutacao.data ?? null,
    /** Chave atual (vai na volta da reautenticação). */
    chave: (): string => chave.current,
    /** Começa outra operação de negócio: chave nova, erro e resultado limpos. */
    reiniciar: () => {
      chave.current = novaChaveDeIdempotencia();
      recusada.current = false;
      mutacao.reset();
    },
    /** Limpa só o erro mostrado, mantendo a mesma operação (e a mesma chave). */
    limparErro: () => {
      if (mutacao.isError) {
        mutacao.reset();
      }
    },
  };
}

export type Operacao<V, R> = ReturnType<typeof useOperacao<V, R>>;
