import { useMutation, useQueryClient } from '@tanstack/react-query';
import { ErroSemConexao, type Laudo } from '@vidacor/api-client';
import { useEffect, useRef, useState } from 'react';
import { chaves } from '../../consultas';
import { useDependencias } from '../../contexto';

export type SituacaoDoRascunho = 'salvo' | 'pendente' | 'salvando' | 'sem-conexao' | 'erro';

/**
 * Salvamento automático do rascunho (R6): espera a pessoa parar de digitar e grava no servidor
 * (`PUT /v1/laudos/{id}/rascunho`). Nada vai para o armazenamento do navegador. Sem conexão, o texto
 * continua na tela e é gravado na próxima tentativa.
 */
export function useRascunho(laudo: Laudo, versaoNumero: number, textoInicial: string) {
  const { api, atrasoDoRascunhoMs = 1200 } = useDependencias();
  const queryClient = useQueryClient();
  const [texto, setTexto] = useState(textoInicial);
  const [situacao, setSituacao] = useState<SituacaoDoRascunho>('salvo');
  const [salvoEm, setSalvoEm] = useState<Date | null>(null);
  const ultimoSalvo = useRef(textoInicial);
  const temporizador = useRef<ReturnType<typeof setTimeout> | null>(null);

  const salvar = useMutation({
    mutationFn: (t: string) => api.laudos.salvarRascunho(laudo.id, { texto: t, versao_numero: versaoNumero }),
    onSuccess: (atual, t) => {
      ultimoSalvo.current = t;
      setSalvoEm(new Date());
      setSituacao((s) => (s === 'salvando' ? 'salvo' : s));
      queryClient.setQueryData(chaves.laudo(laudo.id), atual);
    },
    onError: (erro) => {
      setSituacao(erro instanceof ErroSemConexao ? 'sem-conexao' : 'erro');
    },
  });

  useEffect(
    () => () => {
      if (temporizador.current !== null) clearTimeout(temporizador.current);
    },
    [],
  );

  function gravar(t: string): Promise<boolean> {
    setSituacao('salvando');
    return salvar
      .mutateAsync(t)
      .then(() => true)
      .catch(() => false);
  }

  function mudar(t: string): void {
    setTexto(t);
    setSituacao('pendente');
    if (temporizador.current !== null) clearTimeout(temporizador.current);
    temporizador.current = setTimeout(() => {
      temporizador.current = null;
      void gravar(t);
    }, atrasoDoRascunhoMs);
  }

  /** Grava já o que estiver pendente (antes de assinar). Devolve `false` se não conseguiu. */
  async function salvarAgora(): Promise<boolean> {
    if (temporizador.current !== null) {
      clearTimeout(temporizador.current);
      temporizador.current = null;
    }
    if (texto === ultimoSalvo.current && situacao !== 'sem-conexao' && situacao !== 'erro') {
      return true;
    }
    return gravar(texto);
  }

  return {
    texto,
    mudar,
    situacao,
    salvoEm,
    erro: salvar.error,
    salvarAgora,
    tentarDeNovo: () => void gravar(texto),
  };
}
