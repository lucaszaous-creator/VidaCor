import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { PedidoDesconto } from '@vidacor/api-client';
import { Aviso, Botao, formatarHora, formatarPontosBase } from '@vidacor/design-system';
import { useEffect, useRef, useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

interface Props {
  pacienteId: string;
  pontos: number;
  motivo: string;
  /** Chamado uma vez quando a gerência autoriza: o recebimento leva este ID. */
  aoAutorizar: (autorizacaoId: string) => void;
  /** Pedido cancelado ou recusado: a tela volta ao recebimento sem a autorização. */
  aoEncerrar: () => void;
}

/**
 * Pedido de desconto acima do limite (mockup Caixa: "aguardando a autorização"). O pedido entra na
 * fila de pendências da gerência; a tela acompanha a situação e, autorizado, aplica a autorização
 * sozinha. O total continua sendo calculado no servidor.
 */
export function PedidoDeDesconto({ pacienteId, pontos, motivo, aoAutorizar, aoEncerrar }: Props) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [pedido, setPedido] = useState<PedidoDesconto | null>(null);
  const avisou = useRef(false);
  const pedir = useOperacao(
    (_: undefined, chave: string) => api.pedidosDeDesconto.pedir({ paciente_id: pacienteId, pontos_base: pontos, motivo }, { idempotencyKey: chave }),
    { aoConcluir: setPedido },
  );
  const cancelar = useOperacao((id: string, chave: string) => api.pedidosDeDesconto.cancelar(id, { idempotencyKey: chave }), {
    aoConcluir: () => {
      aoEncerrar();
    },
  });
  const acompanhamento = useQuery({
    queryKey: ['pedido-de-desconto', pedido?.id],
    queryFn: () => api.pedidosDeDesconto.obter(pedido?.id ?? ''),
    enabled: pedido !== null,
    refetchInterval: (q) => (q.state.data?.situacao === 'pedido' || q.state.data === undefined ? 5000 : false),
  });

  useEffect(() => {
    pedir.enviar(undefined);
    // Envia uma vez ao abrir; repetir usa a mesma chave.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const atual = acompanhamento.data ?? pedido;
  useEffect(() => {
    if (atual?.situacao === 'autorizado' && typeof atual.autorizacao_id === 'string' && !avisou.current) {
      avisou.current = true;
      aoAutorizar(atual.autorizacao_id);
    }
  }, [atual, aoAutorizar]);

  if (pedir.erro !== null) {
    return <ErroDoAto erro={pedir.erro} ato="pedir a autorização do desconto" aoTentarDeNovo={() => { pedir.enviar(undefined); }} />;
  }
  if (atual === null) {
    return (
      <Aviso nivel="informativo" titulo="Enviando o pedido à gerência.">
        O total só muda depois da autorização.
      </Aviso>
    );
  }
  const pct = formatarPontosBase(atual.pontos_base);
  const decisor = atual.decidido_por?.nome ?? 'a gerência';
  if (atual.situacao === 'autorizado') {
    return (
      <Aviso nivel="informativo" titulo={`Desconto de ${pct} autorizado por ${decisor}.`}>
        Motivo: {atual.motivo}. Registre o recebimento: o servidor aplica o desconto e confere a autorização.
      </Aviso>
    );
  }
  if (atual.situacao === 'recusado') {
    return (
      <Aviso
        nivel="atencao"
        titulo={`Desconto de ${pct} recusado por ${decisor}.`}
        acoes={
          <Botao pequeno onPress={aoEncerrar}>
            Voltar ao recebimento
          </Botao>
        }
      >
        Motivo: {atual.motivo_decisao}. Nada foi cobrado. Registre sem desconto ou com até o seu limite.
      </Aviso>
    );
  }
  if (atual.situacao === 'cancelado') {
    return null;
  }
  return (
    <div className="col" style={{ gap: 8 }}>
      <Aviso
        nivel="informativo"
        titulo={`Desconto de ${pct} aguardando a gerência.`}
        acoes={
          <Botao
            pequeno
            ocupado={cancelar.enviando}
            onPress={() => {
              cancelar.enviar(atual.id);
              void cliente.invalidateQueries({ queryKey: ['pedido-de-desconto'] });
            }}
          >
            Cancelar pedido
          </Botao>
        }
      >
        O pedido {atual.codigo} está nas pendências da gerência desde {formatarHora(atual.criado_em, FUSO_DA_CLINICA)}. O total só muda depois
        da autorização. Para cobrar o valor cheio, cancele o pedido.
      </Aviso>
      {cancelar.erro !== null && <ErroDoAto erro={cancelar.erro} ato="cancelar o pedido" aoTentarDeNovo={() => { cancelar.enviar(atual.id); }} />}
    </div>
  );
}
