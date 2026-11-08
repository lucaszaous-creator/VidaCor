import type { CalendarDate } from '@internationalized/date';
import {
  DateField,
  DateInput,
  DateSegment,
  FieldError,
  I18nProvider,
  Input,
  Label,
  SearchField,
  Text,
  TextField,
  type TextFieldProps,
} from 'react-aria-components';
import { Icone } from './Icone';

export interface PropsDoCampo extends Omit<TextFieldProps, 'className' | 'children' | 'isInvalid'> {
  rotulo: string;
  /** Formato explicado antes de errar (Estados.dc.html, "campo com erro"). */
  ajuda?: string;
  /** Erro junto do campo, em texto, com o valor mantido. Vem do servidor ou da ajuda de formato. */
  erro?: string | null;
  placeholder?: string;
}

/** Campo de texto do vc10 (`field` com `in`) sobre o TextField do React Aria. */
export function Campo({ rotulo, ajuda, erro, placeholder, ...resto }: PropsDoCampo) {
  const invalido = erro !== undefined && erro !== null && erro !== '';
  return (
    <TextField {...resto} isInvalid={invalido} className="field">
      <Label>{rotulo}</Label>
      <Input className="in" placeholder={placeholder} />
      {ajuda !== undefined && (
        <Text slot="description" className="campo-ajuda">
          {ajuda}
        </Text>
      )}
      <FieldError className="campo-erro">{erro}</FieldError>
    </TextField>
  );
}

export interface PropsDoCampoDeData {
  rotulo: string;
  valor: CalendarDate | null;
  aoMudar: (valor: CalendarDate | null) => void;
  ajuda?: string;
  erro?: string | null;
  obrigatorio?: boolean;
  desabilitado?: boolean;
  nome?: string;
}

/** Data em pt-BR (dd/mm/aaaa) com o DateField do React Aria, digitável por segmento. */
export function CampoDeData({ rotulo, valor, aoMudar, ajuda, erro, obrigatorio, desabilitado, nome }: PropsDoCampoDeData) {
  const invalido = erro !== undefined && erro !== null && erro !== '';
  return (
    <I18nProvider locale="pt-BR">
      <DateField
        className="field"
        value={valor}
        onChange={aoMudar}
        isInvalid={invalido}
        isRequired={obrigatorio ?? false}
        isDisabled={desabilitado ?? false}
        name={nome}
        granularity="day"
      >
        <Label>{rotulo}</Label>
        <DateInput className="in data">{(segmento) => <DateSegment segment={segmento} className="seg-d" />}</DateInput>
        {ajuda !== undefined && (
          <Text slot="description" className="campo-ajuda">
            {ajuda}
          </Text>
        )}
        <FieldError className="campo-erro">{erro}</FieldError>
      </DateField>
    </I18nProvider>
  );
}

export interface PropsDaBusca {
  rotulo: string;
  valor: string;
  aoMudar: (valor: string) => void;
  placeholder?: string;
  /** Atalho mostrado à direita (`kbd`), como "Ctrl K". */
  atalho?: string;
}

/** Busca do vc10 (`search` com `kbd`). O rótulo fica só para leitor de tela (`sr`). */
export function Busca({ rotulo, valor, aoMudar, placeholder, atalho }: PropsDaBusca) {
  return (
    <SearchField value={valor} onChange={aoMudar} aria-label={rotulo} className="search">
      <Icone nome="busca" />
      <Input placeholder={placeholder} />
      {atalho !== undefined && (
        <span className="kbd" aria-hidden="true">
          {atalho}
        </span>
      )}
    </SearchField>
  );
}
