import { useQueryClient } from '@tanstack/react-query';
import type { AutorizacaoDeDesconto, PedidoDesconto } from '@vidacor/api-client';
import { Aviso, Botao, DialogoDeFormulario, formatarData, formatarDataHora, formatarPontosBase } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar, type Retomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { Recusa } from './Recusa';
import { chavesDasPendencias } from './TelaPendencias';

/** Autorizar (reautenticação) ou recusar com motivo um pedido de desconto do caixa (FC-CXA-05). */
export function DecisaoDeDesconto({ pedido, retomada }: { pedido: PedidoDesconto; retomada: Retomada | null }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const reautenticar = useReautenticar();
  const voltouDaReautenticacao = retomada?.retomar === 'autorizar';
  const [confirmando, setConfirmando] = useState(voltouDaReautenticacao);
  const [feito, setFeito] = useState<AutorizacaoDeDesconto | PedidoDesconto | null>(null);
  const atualizar = () => void cliente.invalidateQueries({ queryKey: chavesDasPendencias.descontos });
  const autorizar = useOperacao(
    (_: undefined, chave: string) =>
      api.autorizacoesDeDesconto.criar(
        { paciente_id: pedido.paciente.id, pontos_base: pedido.pontos_base, motivo: pedido.motivo, solicitante_id: pedido.solicitante.id, pedido_id: pedido.id },
        { idempotencyKey: chave },
      ),
    {
      chaveInicial: voltouDaReautenticacao ? retomada.chave : undefined,
      aoConcluir: (a) => {
        setFeito(a);
        setConfirmando(false);
      },
      aoFalhar: () => {
        setConfirmando(false);
      },
    },
  );
  const recusar = useOperacao((motivo: string, chave: string) => api.pedidosDeDesconto.recusar(pedido.id, { motivo }, { idempotencyKey: chave }), {
    aoConcluir: setFeito,
  });
  const pct = formatarPontosBase(pedido.pontos_base);
  const nome = pedido.paciente.nome;

  if (feito !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={'valida_ate' in feito ? `Desconto de ${pct} autorizado para ${nome}.` : `Desconto recusado para ${nome}.`}
        acoes={
          <Botao pequeno onPress={atualizar}>
            Próxima pendência
          </Botao>
        }
      >
        {'valida_ate' in feito
          ? `A tela do caixa recebe a autorização sozinha; vale até ${formatarDataHora(feito.valida_ate, FUSO_DA_CLINICA)} e uma vez só.`
          : `${pedido.solicitante.nome} vê o motivo no caixa.`}{' '}
        O lastro grava quem pediu, quem decidiu e o motivo.
      </Aviso>
    );
  }

  return (
    <div className="col" style={{ gap: 12 }}>
      <h2 className="t15 b" style={{ margin: 0 }}>
        Desconto de {pct} pedido por {pedido.solicitante.nome}
      </h2>
      <p className="t14" style={{ margin: 0, lineHeight: 1.6 }}>
        Paciente <b>{nome}</b>, nascimento <b className="num">{formatarData(pedido.paciente.nascimento)}</b>. Motivo informado: {pedido.motivo}. Pedido{' '}
        <span className="num">{pedido.codigo}</span>.
      </p>
      {autorizar.erro !== null && (
        <ErroDoAto
          erro={autorizar.erro}
          ato="autorizar o desconto"
          aoReautenticar={() => {
            reautenticar('autorizar', autorizar.chave(), {}, () => {
              autorizar.enviar(undefined);
            });
          }}
          aoTentarDeNovo={() => {
            autorizar.enviar(undefined);
          }}
        />
      )}
      <div className="row" style={{ gap: 8 }}>
        <Botao
          variante="principal"
          onPress={() => {
            setConfirmando(true);
          }}
        >
          Autorizar {pct}
        </Botao>
      </div>
      <Recusa oQue="o desconto" op={recusar} />
      <DialogoDeFormulario
        aberto={confirmando}
        titulo={`Autorizar ${pct} de desconto para ${nome}?`}
        paciente={{ nome, nascimento: formatarData(pedido.paciente.nascimento) }}
        rotuloConfirmar="Autorizar desconto"
        ocupado={autorizar.enviando}
        aoFechar={() => {
          setConfirmando(false);
        }}
        aoConfirmar={() => {
          autorizar.enviar(undefined);
        }}
      >
        <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
          {nome}, nascimento {formatarData(pedido.paciente.nascimento)}. A autorização é de uso único, vale por 2 horas e só para este paciente. Pede a
          sua senha e o segundo fator se o último login tiver mais de 5 minutos.
        </p>
      </DialogoDeFormulario>
    </div>
  );
}
