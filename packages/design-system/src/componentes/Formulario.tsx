import { useId, type ReactNode } from 'react';
import {
  CheckboxButton,
  CheckboxField,
  FieldError,
  Label,
  RadioButton,
  RadioField,
  RadioGroup,
  Text,
  TextArea,
  TextField,
  type TextFieldProps,
} from 'react-aria-components';
import { Icone } from './Icone';

/* Controles de formulário que as telas da Fase 1 usam além do Campo e do CampoDeData. Todos com
   rótulo visível, erro em texto junto do campo e teclado completo (R10). */

export interface PropsDaAreaDeTexto extends Omit<TextFieldProps, 'className' | 'children' | 'isInvalid'> {
  rotulo: string;
  ajuda?: string;
  erro?: string | null;
  linhas?: number;
  placeholder?: string;
}

/** Texto longo (motivo, justificativa, texto do laudo). O conteúdo é sempre texto, nunca HTML. */
export function AreaDeTexto({ rotulo, ajuda, erro, linhas = 4, placeholder, ...resto }: PropsDaAreaDeTexto) {
  const invalido = erro !== undefined && erro !== null && erro !== '';
  return (
    <TextField {...resto} isInvalid={invalido} className="field">
      <Label>{rotulo}</Label>
      <TextArea className="in" rows={linhas} placeholder={placeholder} />
      {ajuda !== undefined && (
        <Text slot="description" className="campo-ajuda">
          {ajuda}
        </Text>
      )}
      <FieldError className="campo-erro">{erro}</FieldError>
    </TextField>
  );
}

export interface PropsDaCaixaDeMarcar {
  rotulo: string;
  marcado: boolean;
  aoMudar: (marcado: boolean) => void;
  /** Complemento abaixo do rótulo. */
  detalhe?: string;
  erro?: string | null;
  desabilitado?: boolean;
}

/** Caixa de marcar do sistema (alvo de 44 px, foco visível). */
export function CaixaDeMarcar({ rotulo, marcado, aoMudar, detalhe, erro, desabilitado = false }: PropsDaCaixaDeMarcar) {
  const idErro = useId();
  const invalido = erro !== undefined && erro !== null && erro !== '';
  return (
    <div className="col" style={{ gap: 4 }}>
      <CheckboxField
        isSelected={marcado}
        onChange={aoMudar}
        isDisabled={desabilitado}
        isInvalid={invalido}
        aria-describedby={invalido ? idErro : undefined}
      >
        <CheckboxButton className="ck">
          <span className="ck-caixa" aria-hidden="true">
            <Icone nome="confere" tamanho={14} traco={2.6} />
          </span>
          <span className="col" style={{ gap: 2 }}>
            <span className="t14" style={{ color: 'var(--ink)', fontWeight: 550 }}>
              {rotulo}
            </span>
            {detalhe !== undefined && <span className="t13 mute">{detalhe}</span>}
          </span>
        </CheckboxButton>
      </CheckboxField>
      {invalido && (
        <span id={idErro} className="campo-erro">
          {erro}
        </span>
      )}
    </div>
  );
}

export interface PropsDoSimOuNao {
  pergunta: string;
  /** `null` enquanto ninguém respondeu: nada vem pré-marcado (R11). */
  valor: boolean | null;
  aoMudar: (valor: boolean) => void;
  detalhe?: string;
  erro?: string | null;
  desabilitado?: boolean;
}

/** Pergunta de sim ou não (triagem). Setas trocam a resposta, como em qualquer grupo de rádio. */
export function SimOuNao({ pergunta, valor, aoMudar, detalhe, erro, desabilitado = false }: PropsDoSimOuNao) {
  const invalido = erro !== undefined && erro !== null && erro !== '';
  return (
    <RadioGroup
      className="sn"
      value={valor === null ? null : valor ? 'sim' : 'nao'}
      onChange={(v) => {
        aoMudar(v === 'sim');
      }}
      orientation="horizontal"
      isDisabled={desabilitado}
      isInvalid={invalido}
    >
      <div className="col" style={{ gap: 2, flex: 1, minWidth: 0 }}>
        <Label className="t14" style={{ color: 'var(--ink)', fontWeight: 550 }}>
          {pergunta}
        </Label>
        {detalhe !== undefined && (
          <Text slot="description" className="t13 mute">
            {detalhe}
          </Text>
        )}
        <FieldError className="campo-erro">{erro}</FieldError>
      </div>
      <div className="seg sn-opcoes">
        <RadioField value="sim">
          <RadioButton className="sn-op">Sim</RadioButton>
        </RadioField>
        <RadioField value="nao">
          <RadioButton className="sn-op">Não</RadioButton>
        </RadioField>
      </div>
    </RadioGroup>
  );
}

export interface OpcaoDaEscolha {
  valor: string;
  rotulo: string;
}

export interface PropsDaEscolha {
  rotulo: string;
  valor: string;
  aoMudar: (valor: string) => void;
  opcoes: OpcaoDaEscolha[];
  /** Primeira opção vazia ("Escolha o motivo"); sem ela, a primeira opção vem escolhida. */
  vazio?: string;
  erro?: string | null;
  ajuda?: string;
  desabilitado?: boolean;
}

/** Lista de escolha nativa (funciona com leitor de tela e teclado sem esforço extra). */
export function Escolha({ rotulo, valor, aoMudar, opcoes, vazio, erro, ajuda, desabilitado = false }: PropsDaEscolha) {
  const id = useId();
  const idAjuda = useId();
  const idErro = useId();
  const invalido = erro !== undefined && erro !== null && erro !== '';
  const descritores = [ajuda === undefined ? '' : idAjuda, invalido ? idErro : ''].filter(Boolean).join(' ');
  return (
    <div className="field">
      <label htmlFor={id}>{rotulo}</label>
      <select
        id={id}
        className="in"
        value={valor}
        disabled={desabilitado}
        aria-invalid={invalido}
        aria-describedby={descritores === '' ? undefined : descritores}
        onChange={(ev) => {
          aoMudar(ev.target.value);
        }}
      >
        {vazio !== undefined && <option value="">{vazio}</option>}
        {opcoes.map((o) => (
          <option key={o.valor} value={o.valor}>
            {o.rotulo}
          </option>
        ))}
      </select>
      {ajuda !== undefined && (
        <span id={idAjuda} className="campo-ajuda">
          {ajuda}
        </span>
      )}
      {invalido && (
        <span id={idErro} className="campo-erro">
          {erro}
        </span>
      )}
    </div>
  );
}

export interface PropsDaListaDeDados {
  itens: { rotulo: string; valor: ReactNode }[];
  /** Largura da coluna dos rótulos. */
  larguraDoRotulo?: number;
}

/** Pares rótulo e valor (`.kv` do vc10). */
export function ListaDeDados({ itens, larguraDoRotulo = 120 }: PropsDaListaDeDados) {
  return (
    <dl className="kv" style={{ margin: 0, gridTemplateColumns: `${String(larguraDoRotulo)}px minmax(0, 1fr)` }}>
      {itens.map((i) => (
        <div key={i.rotulo} style={{ display: 'contents' }}>
          <dt>{i.rotulo}</dt>
          <dd>{i.valor}</dd>
        </div>
      ))}
    </dl>
  );
}
