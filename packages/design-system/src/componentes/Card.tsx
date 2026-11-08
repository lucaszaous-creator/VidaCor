import { useId, type CSSProperties, type ReactNode } from 'react';

export interface PropsDoCard {
  /** Título do cartão (`card-h`). Sem título, passe `rotulo` para nomear a região. */
  titulo?: string;
  /** Texto curto ao lado do título. Use pouco: subtítulo em toda caixa é proibido (docs/05). */
  complemento?: string;
  /** Uma próxima ação por cartão (regra 2 do docs/05), à direita do título. */
  acao?: ReactNode;
  rotulo?: string;
  /** `pane`: painel que cabe na janela e rola por dentro (`.card.pane`). */
  painel?: boolean;
  className?: string;
  style?: CSSProperties;
  children: ReactNode;
}

export function Card({ titulo, complemento, acao, rotulo, painel = false, className, style, children }: PropsDoCard) {
  const idTitulo = useId();
  const classe = ['card', painel ? 'pane' : '', className ?? ''].filter(Boolean).join(' ');
  return (
    <section
      className={classe}
      style={style}
      aria-labelledby={titulo === undefined ? undefined : idTitulo}
      aria-label={titulo === undefined ? rotulo : undefined}
    >
      {titulo !== undefined && (
        <div className="card-h">
          <div>
            <h2 id={idTitulo}>{titulo}</h2>
            {complemento !== undefined && <div className="sub">{complemento}</div>}
          </div>
          {acao !== undefined && <div style={{ marginLeft: 'auto' }}>{acao}</div>}
        </div>
      )}
      {children}
    </section>
  );
}
