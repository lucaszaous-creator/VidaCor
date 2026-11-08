import { Botao, Escolha, formatarHora } from '@vidacor/design-system';
import { useState } from 'react';
import { useDependencias } from '../contexto';
import { useOperacao } from '../operacao';
import { FUSO_DA_CLINICA } from '../telas/agenda/grade';
import { ErroDoAto } from './ErroDoAto';

const GUICHES = Array.from({ length: 20 }, (_, k) => ({ valor: String(k + 1), rotulo: `Guichê ${String(k + 1)}` }));

/**
 * Guichê desta estação, lembrado só em memória enquanto a tela está aberta: nada vai para o
 * armazenamento do navegador (SEC-CLI-3). Não é segurança; o servidor confere o perfil.
 */
let guicheDaEstacao = '1';

export interface PropsDeChamarSenha {
  atendimentoId: string;
  /** Recepção chama ao guichê; o técnico, à sala do atendimento. */
  destino: 'guiche' | 'sala';
  /** Nome do paciente, para o rótulo acessível quando há vários botões na lista. */
  nome: string;
  compacto?: boolean;
}

/**
 * Chama a senha na TV (Telao.dc.html). Cada toque é uma chamada nova (rechamar é chamar de novo); o
 * duplo clique não duplica (Idempotency-Key). O texto da TV é decidido pelo servidor.
 */
export function ChamarSenha({ atendimentoId, destino, nome, compacto = false }: PropsDeChamarSenha) {
  const { api } = useDependencias();
  const [guiche, setGuiche] = useState(() => guicheDaEstacao);
  const op = useOperacao((_: undefined, chave: string) =>
    api.atendimentos.chamar(atendimentoId, destino === 'guiche' ? { destino, guiche: Number(guiche) } : { destino }, { idempotencyKey: chave }),
  );
  const feita = op.resultado;
  return (
    <div className="col" style={{ gap: 6 }}>
      <div className="row wrap" style={{ gap: 8, alignItems: 'flex-end' }}>
        {destino === 'guiche' && !compacto && (
          <div style={{ width: 130 }}>
            <Escolha
              rotulo="Meu guichê"
              valor={guiche}
              opcoes={GUICHES}
              aoMudar={(v) => {
                setGuiche(v);
                guicheDaEstacao = v;
                op.reiniciar();
              }}
            />
          </div>
        )}
        <Botao
          pequeno
          ocupado={op.enviando}
          aria-label={`${feita === null ? 'Chamar' : 'Chamar de novo'} ${nome} ${destino === 'guiche' ? `ao guichê ${guiche}` : 'para a sala'}`}
          onPress={() => {
            if (feita !== null) op.reiniciar();
            op.enviar(undefined);
          }}
        >
          {feita === null ? 'Chamar na TV' : 'Chamar de novo'}
        </Botao>
      </div>
      <div aria-live="polite" className="t13 mute">
        {feita !== null && `Senha ${feita.senha} chamada para ${feita.destino} às ${formatarHora(feita.chamada_em, FUSO_DA_CLINICA)}.`}
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="chamar a senha" />}
    </div>
  );
}
