import { forwardRef, type ReactNode } from 'react';
import { Button, type ButtonProps } from 'react-aria-components';
import { Icone, type NomeDoIcone } from './Icone';

export type VarianteDoBotao = 'principal' | 'azul' | 'padrao' | 'discreto';

const CLASSE: Record<VarianteDoBotao, string> = {
  principal: 'btn btn-p',
  azul: 'btn btn-b',
  padrao: 'btn',
  discreto: 'btn btn-g',
};

export interface PropsDoBotao extends Omit<ButtonProps, 'className' | 'children' | 'style'> {
  variante?: VarianteDoBotao;
  pequeno?: boolean;
  /** Enquanto o servidor não responde: botão ocupado, sem segundo envio (G3). */
  ocupado?: boolean;
  larguraTotal?: boolean;
  children: ReactNode;
}

/** Botão do vc10 (`btn`, `btn-p` marinho, `btn-b` azul) com o comportamento do React Aria. */
export const Botao = forwardRef<HTMLButtonElement, PropsDoBotao>(function Botao(
  { variante = 'padrao', pequeno = false, ocupado = false, larguraTotal = false, children, ...resto },
  ref,
) {
  const classe = `${CLASSE[variante]}${pequeno ? ' btn-sm' : ''}`;
  return (
    <Button
      {...resto}
      ref={ref}
      className={classe}
      isPending={ocupado}
      style={larguraTotal ? { width: '100%' } : undefined}
    >
      {children}
    </Button>
  );
});

export interface PropsDoBotaoDeIcone extends Omit<ButtonProps, 'className' | 'children' | 'aria-label'> {
  icone: NomeDoIcone;
  /** Obrigatório: botão só com ícone precisa de nome para o leitor de tela. */
  rotulo: string;
  /** `icon-btn` (redondo com borda), `xb` (fechar da gaveta) ou `more` (mais opções). */
  estilo?: 'redondo' | 'fechar' | 'mais';
}

const CLASSE_ICONE = { redondo: 'icon-btn', fechar: 'xb', mais: 'more' } as const;

export const BotaoDeIcone = forwardRef<HTMLButtonElement, PropsDoBotaoDeIcone>(function BotaoDeIcone(
  { icone, rotulo, estilo = 'redondo', ...resto },
  ref,
) {
  return (
    <Button {...resto} ref={ref} aria-label={rotulo} className={CLASSE_ICONE[estilo]}>
      <Icone nome={icone} />
    </Button>
  );
});
