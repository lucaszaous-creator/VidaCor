import { useId, type KeyboardEvent, type ReactNode } from 'react';
import { BotaoDeIcone } from './Botao';

export interface PropsDaGaveta {
  /** Linha pequena acima do título (horário, local). */
  sobreTitulo?: string;
  titulo: string;
  /** Linha abaixo do título; em gaveta de paciente, o nascimento (dois identificadores). */
  subTitulo?: string;
  rotulo: string;
  /** Ações no cabeçalho, antes do fechar (por exemplo, o botão de mais opções). */
  acoesDoCabecalho?: ReactNode;
  rodape?: ReactNode;
  aoFechar: () => void;
  children: ReactNode;
}

/**
 * Gaveta de detalhe (Agenda.dc.html e Recepcao.dc.html): o secundário vai para a gaveta, não para
 * baixo da tela. O conteúdo rola por dentro; Esc fecha.
 */
export function Gaveta({ sobreTitulo, titulo, subTitulo, rotulo, acoesDoCabecalho, rodape, aoFechar, children }: PropsDaGaveta) {
  const idTitulo = useId();
  function aoTeclar(ev: KeyboardEvent<HTMLElement>): void {
    if (ev.key === 'Escape' && !ev.defaultPrevented) {
      ev.stopPropagation();
      aoFechar();
    }
  }
  return (
    <aside className="drw" aria-label={rotulo} onKeyDown={aoTeclar}>
      <div className="drw-h">
        <div style={{ flex: 1, minWidth: 0 }}>
          {sobreTitulo !== undefined && <div className="t13 mute">{sobreTitulo}</div>}
          <h2 id={idTitulo} style={{ fontSize: 19, fontWeight: 650, marginTop: 2 }}>
            {titulo}
          </h2>
          {subTitulo !== undefined && (
            <div className="t14 num" style={{ color: 'var(--ink2)' }}>
              {subTitulo}
            </div>
          )}
        </div>
        {acoesDoCabecalho}
        <BotaoDeIcone icone="fechar" estilo="fechar" rotulo="Fechar detalhe" onPress={aoFechar} />
      </div>
      <div className="scroll drw-c">{children}</div>
      {rodape !== undefined && <div className="drw-r">{rodape}</div>}
    </aside>
  );
}
