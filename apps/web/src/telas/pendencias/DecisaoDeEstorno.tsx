import { useQueryClient } from '@tanstack/react-query';
import type { EstornoDetalhado } from '@vidacor/api-client';
import { Aviso, Botao, DialogoDeFormulario, formatarData, formatarDataHora, formatarDinheiro } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar, type Retomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { Recusa } from './Recusa';
import { chavesDasPendencias } from './TelaPendencias';

const FORMAS: Record<string, string> = { pix: 'PIX', dinheiro: 'dinheiro', cartao: 'cartão' };

/** Aprovar (reautenticação, irreversível) ou recusar com motivo um estorno (FC-CXA-09). */
export function DecisaoDeEstorno({ estorno, retomada }: { estorno: EstornoDetalhado; retomada: Retomada | null }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const reautenticar = useReautenticar();
  const voltouDaReautenticacao = retomada?.retomar === 'aprovar-estorno';
  const [confirmando, setConfirmando] = useState(voltouDaReautenticacao);
  const [feito, setFeito] = useState<'aprovado' | 'recusado' | null>(null);
  const aprovar = useOperacao((_: undefined, chave: string) => api.estornos.aprovar(estorno.id, { idempotencyKey: chave }), {
    chaveInicial: voltouDaReautenticacao ? retomada.chave : undefined,
    aoConcluir: () => {
      setFeito('aprovado');
      setConfirmando(false);
    },
    aoFalhar: () => {
      setConfirmando(false);
    },
  });
  const recusar = useOperacao((motivo: string, chave: string) => api.estornos.recusar(estorno.id, { motivo }, { idempotencyKey: chave }), {
    aoConcluir: () => {
      setFeito('recusado');
    },
  });
  const nome = estorno.paciente.nome;
  const valor = formatarDinheiro(estorno.total_centavos);

  if (feito !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={feito === 'aprovado' ? `Estorno de ${valor} aprovado para ${nome}.` : `Estorno recusado para ${nome}.`}
        acoes={
          <Botao pequeno onPress={() => void cliente.invalidateQueries({ queryKey: chavesDasPendencias.estornos })}>
            Próxima pendência
          </Botao>
        }
      >
        {feito === 'aprovado'
          ? estorno.forma === 'pix'
            ? 'A devolução segue pelo provedor do PIX.'
            : 'Devolva o valor ao paciente pelo caixa.'
          : `${estorno.pedido_por.nome} vê o motivo e pode pedir de novo se for o caso.`}{' '}
        O lastro grava quem pediu e quem decidiu.
      </Aviso>
    );
  }

  return (
    <div className="col" style={{ gap: 12 }}>
      <h2 className="t15 b" style={{ margin: 0 }}>
        Estorno de {valor} pedido por {estorno.pedido_por.nome}
      </h2>
      <p className="t14" style={{ margin: 0, lineHeight: 1.6 }}>
        Paciente <b>{nome}</b>, nascimento <b className="num">{formatarData(estorno.paciente.nascimento)}</b>. Recebimento{' '}
        <span className="num">{estorno.recebimento_codigo}</span> em {FORMAS[estorno.forma] ?? estorno.forma}
        {typeof estorno.pago_em === 'string' ? `, pago em ${formatarDataHora(estorno.pago_em, FUSO_DA_CLINICA)}` : ''}. Motivo: {estorno.motivo}.
      </p>
      {aprovar.erro !== null && (
        <ErroDoAto
          erro={aprovar.erro}
          ato="aprovar o estorno"
          aoReautenticar={() => {
            reautenticar('aprovar-estorno', aprovar.chave(), {}, () => {
              aprovar.enviar(undefined);
            });
          }}
          aoTentarDeNovo={() => {
            aprovar.enviar(undefined);
          }}
        />
      )}
      <div className="row">
        <Botao
          variante="principal"
          onPress={() => {
            setConfirmando(true);
          }}
        >
          Aprovar estorno
        </Botao>
      </div>
      <Recusa oQue="o estorno" op={recusar} />
      <DialogoDeFormulario
        aberto={confirmando}
        irreversivel
        titulo={`Estornar ${valor} para ${nome}?`}
        paciente={{ nome, nascimento: formatarData(estorno.paciente.nascimento) }}
        rotuloConfirmar="Aprovar estorno"
        ocupado={aprovar.enviando}
        aoFechar={() => {
          setConfirmando(false);
        }}
        aoConfirmar={() => {
          aprovar.enviar(undefined);
        }}
      >
        <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
          {nome}, nascimento {formatarData(estorno.paciente.nascimento)}. O valor volta ao paciente e não há como desfazer. Quem pediu o estorno não pode
          aprovar.
        </p>
      </DialogoDeFormulario>
    </div>
  );
}
