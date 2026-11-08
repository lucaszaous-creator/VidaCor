import { AreaDeTexto, Botao } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import type { Operacao } from '../../operacao';

/** Recusa com motivo que o caixa lê (mínimo 5 caracteres, conferido também no servidor). */
export function Recusa<R>({ oQue, op }: { oQue: string; op: Operacao<string, R> }) {
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const curto = motivo.trim().length < 5;
  return (
    <form
      className="col"
      style={{ gap: 8 }}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (!curto) op.enviar(motivo.trim());
      }}
    >
      <AreaDeTexto
        rotulo={`Motivo da recusa (o caixa lê)`}
        value={motivo}
        onChange={setMotivo}
        linhas={2}
        erro={tentou && curto ? 'Escreva o motivo, com pelo menos 5 letras.' : null}
        isDisabled={op.enviando}
      />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato={`recusar ${oQue}`} aoTentarDeNovo={() => { op.enviar(motivo.trim()); }} />}
      <div className="row">
        <Botao type="submit" ocupado={op.enviando}>
          Recusar
        </Botao>
      </div>
    </form>
  );
}
