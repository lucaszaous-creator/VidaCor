import { useNavigate } from '@tanstack/react-router';
import { ErroDaApi, ErroSemConexao, ehReautenticacao } from '@vidacor/api-client';
import { Aviso, Botao } from '@vidacor/design-system';
import type { ReactNode } from 'react';

export interface PropsDoErroDoAto {
  erro: unknown;
  /** O ato, no infinitivo, para as mensagens ("assinar o laudo"). */
  ato: string;
  /** Leva à reautenticação mantendo a chave (só para atos sensíveis). */
  aoReautenticar?: () => void;
  /** O que NÃO volta depois da reautenticação e precisará ser digitado de novo. */
  naoPreservado?: string;
  /** Reenvio com a mesma chave quando a rede caiu. */
  aoTentarDeNovo?: () => void;
  /** Ações próprias do erro (ex.: horários sugeridos, pedir autorização). */
  acoes?: ReactNode;
  /** Complemento que a tela sabe do erro (ex.: o procedimento recusado). */
  complemento?: ReactNode;
}

/**
 * Mostra o resultado de um ato que o servidor não confirmou, sempre no formato R8: o que aconteceu,
 * o que foi feito e o que fazer agora. O texto vem do servidor (`detail`) com o código de suporte.
 * A tela nunca mente: sem resposta, diz que não sabe se foi registrado.
 */
export function ErroDoAto({ erro, ato, aoReautenticar, naoPreservado, aoTentarDeNovo, acoes, complemento }: PropsDoErroDoAto) {
  const navegar = useNavigate();

  if (ehReautenticacao(erro) && aoReautenticar !== undefined) {
    return (
      <Aviso
        nivel="atencao"
        titulo={`Confirme sua senha e o segundo fator para ${ato}.`}
        codigoDeSuporte={erro.traceId}
        acoes={
          <Botao pequeno variante="principal" onPress={aoReautenticar}>
            Confirmar identidade
          </Botao>
        }
      >
        {erro.problema.detail} A confirmação abre numa janela à parte; o que você digitou continua aqui e,
        confirmada a identidade, a tela repete o pedido com a mesma chave, sem duplicar.
        {naoPreservado !== undefined &&
          ` Se o navegador bloquear a janela, a tela sai para confirmar e será preciso digitar de novo ${naoPreservado}.`}
      </Aviso>
    );
  }

  if (erro instanceof ErroSemConexao) {
    return (
      <Aviso
        nivel="atencao"
        titulo="Não foi possível falar com o servidor."
        acoes={
          aoTentarDeNovo === undefined ? undefined : (
            <Botao pequeno onPress={aoTentarDeNovo}>
              Tentar de novo
            </Botao>
          )
        }
      >
        Não dá para saber se o pedido para {ato} foi registrado. O que você digitou continua aqui. Tente de novo: o
        reenvio não duplica.
      </Aviso>
    );
  }

  // 401 de login recusado (portal) não é sessão terminada: mostra a mensagem do servidor.
  if (erro instanceof ErroDaApi && erro.status === 401 && erro.codigo !== 'login-nao-confere') {
    return (
      <Aviso
        nivel="bloqueante"
        titulo="Sua sessão terminou."
        codigoDeSuporte={erro.traceId}
        acoes={
          <Botao pequeno onPress={() => void navegar({ to: '/entrar' })}>
            Entrar de novo
          </Botao>
        }
      >
        Nada foi feito. Entre de novo e repita o pedido.
      </Aviso>
    );
  }

  if (erro instanceof ErroDaApi) {
    return (
      <Aviso nivel="bloqueante" codigoDeSuporte={erro.traceId} acoes={acoes}>
        {erro.problema.detail}
        {complemento !== undefined && <> {complemento}</>}
      </Aviso>
    );
  }

  return (
    <Aviso nivel="bloqueante">
      Algo deu errado nesta tela. Nada foi confirmado. Recarregue a página; se continuar, avise a TI.
    </Aviso>
  );
}
