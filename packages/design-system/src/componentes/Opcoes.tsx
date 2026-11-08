import { CheckboxButton, CheckboxField, CheckboxGroup, Label, RadioButton, RadioField, RadioGroup } from 'react-aria-components';
import type { OpcaoDaEscolha } from './Formulario';

/* Grupos de opções em pílula (Laudo-Estruturado.dc.html, `.grp` e `.opt`): rótulo à esquerda e as
   opções lado a lado, quebrando linha quando faltar espaço. Setas, espaço e Tab do React Aria. */

export interface PropsDoGrupoDeOpcoes {
  rotulo: string;
  /** `null` enquanto nada foi escolhido: nada vem pré-marcado. */
  valor: string | null;
  aoMudar: (valor: string) => void;
  opcoes: OpcaoDaEscolha[];
  desabilitado?: boolean;
}

/** Uma opção entre várias (grupo de rádio). */
export function GrupoDeOpcoes({ rotulo, valor, aoMudar, opcoes, desabilitado = false }: PropsDoGrupoDeOpcoes) {
  return (
    <RadioGroup className="grp-op" value={valor} onChange={aoMudar} orientation="horizontal" isDisabled={desabilitado}>
      <Label className="grp-op-rotulo">{rotulo}</Label>
      <div className="seg">
        {opcoes.map((o) => (
          <RadioField key={o.valor} value={o.valor}>
            <RadioButton className="sn-op">{o.rotulo}</RadioButton>
          </RadioField>
        ))}
      </div>
    </RadioGroup>
  );
}

export interface PropsDasOpcoesMultiplas {
  rotulo: string;
  valores: string[];
  aoMudar: (valores: string[]) => void;
  opcoes: OpcaoDaEscolha[];
  desabilitado?: boolean;
}

/** Várias opções ao mesmo tempo (grupo de caixas de marcar com o mesmo visual). */
export function OpcoesMultiplas({ rotulo, valores, aoMudar, opcoes, desabilitado = false }: PropsDasOpcoesMultiplas) {
  return (
    <CheckboxGroup className="grp-op" value={valores} onChange={aoMudar} isDisabled={desabilitado}>
      <Label className="grp-op-rotulo">{rotulo}</Label>
      <div className="seg">
        {opcoes.map((o) => (
          <CheckboxField key={o.valor} value={o.valor}>
            <CheckboxButton className="sn-op">{o.rotulo}</CheckboxButton>
          </CheckboxField>
        ))}
      </div>
    </CheckboxGroup>
  );
}
