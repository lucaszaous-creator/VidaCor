import type { CalendarDate } from '@internationalized/date';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
  ErroDaApi,
  ErroSemConexao,
  novaChaveDeIdempotencia,
  type Atendimento,
  type ItemDaAgenda,
} from '@vidacor/api-client';
import { useRef, useState } from 'react';
import { chaves } from '../../consultas';
import { useDependencias } from '../../contexto';
import { erroParaTela, type ErroParaTela } from '../../erros';

export interface ErrosDosCampos {
  nome: string | null;
  nascimento: string | null;
}

/**
 * Estado do check-in de um agendamento. A tela só pede: quem confere nome e nascimento contra o
 * cadastro e gera atendimento e accession é o servidor (G1). A `Idempotency-Key` nasce uma vez por
 * operação e é reaproveitada em toda retentativa (G3); só é trocada depois que o servidor recusou
 * (4xx), porque aí o próximo envio é outra tentativa, com outros dados.
 */
export function useCheckIn(item: ItemDaAgenda) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const [nome, setNome] = useState('');
  const [nascimento, setNascimento] = useState<CalendarDate | null>(null);
  const [tentou, setTentou] = useState(false);
  const [confirmando, setConfirmando] = useState(false);
  const chave = useRef<string>(novaChaveDeIdempotencia());
  const chaveRecusada = useRef(false);

  const envio = useMutation<Atendimento, unknown, { nome: string; nascimento: string }>({
    mutationFn: (confirmacao) =>
      api.atendimentos.criar({ agendamento_id: item.id, confirmacao }, { idempotencyKey: chave.current }),
    onSuccess: async () => {
      setConfirmando(false);
      await queryClient.invalidateQueries({ queryKey: chaves.agendaTodas });
    },
    onError: async (erro) => {
      setConfirmando(false);
      if (erro instanceof ErroDaApi && erro.status < 500) {
        chaveRecusada.current = true;
      }
      if (erro instanceof ErroDaApi && erro.codigo === 'checkin-ja-realizado') {
        await queryClient.invalidateQueries({ queryKey: chaves.agendaTodas });
      }
    },
  });

  // Ajuda de formato, antes de enviar (Estados.dc.html, "campo com erro"). A decisão é do servidor.
  const errosDosCampos: ErrosDosCampos = {
    nome: tentou && nome.trim() === '' ? 'Digite o nome completo como está no documento.' : null,
    nascimento: tentou && nascimento === null ? 'Digite a data de nascimento completa: dia, mês e ano.' : null,
  };

  function pedirConfirmacao(): void {
    setTentou(true);
    if (nome.trim() !== '' && nascimento !== null) {
      setConfirmando(true);
    }
  }

  function enviar(): void {
    if (nascimento === null) {
      return;
    }
    if (chaveRecusada.current) {
      chave.current = novaChaveDeIdempotencia();
      chaveRecusada.current = false;
    }
    envio.mutate({ nome: nome.trim(), nascimento: nascimento.toString() });
  }

  const erro: ErroParaTela | null = envio.isError ? erroParaTela(envio.error) : null;

  return {
    nome,
    setNome,
    nascimento,
    setNascimento,
    errosDosCampos,
    confirmando,
    cancelarConfirmacao: () => {
      setConfirmando(false);
    },
    pedirConfirmacao,
    enviar,
    enviando: envio.isPending,
    atendimento: envio.data ?? null,
    erro,
    /** Sem conexão: não se sabe se o servidor gravou; reenviar com a mesma chave não duplica. */
    semConexao: envio.error instanceof ErroSemConexao,
  };
}
