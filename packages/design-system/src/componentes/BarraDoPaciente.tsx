import type { ReactNode } from 'react';
import { Avatar } from './Avatar';

export interface PropsDaBarraDoPaciente {
  nome: string;
  /** Já formatado (dd/mm/aaaa). Fica em destaque e nunca é cortado (R2). */
  nascimento: string;
  /** Complemento após o nascimento ("72 anos, masculino"), quando o servidor mandar. */
  complementoDoNascimento?: string;
  /** Alergia em vermelho cheio, nunca cortada. Só quando o servidor mandar. */
  alergia?: string;
  /** Resumo cortável com dica: exame, local, convênio. */
  resumo?: string;
  acoes?: ReactNode;
}

/**
 * Barra do paciente em foco (Barra-Paciente.dc.html), fixa no topo de toda tela clínica, em uma
 * linha: os dois identificadores à esquerda, o resumo no meio, ações à direita.
 */
export function BarraDoPaciente({ nome, nascimento, complementoDoNascimento, alergia, resumo, acoes }: PropsDaBarraDoPaciente) {
  return (
    <section
      aria-label={`Paciente em foco: ${nome}, nascimento ${nascimento}`}
      style={{
        display: 'grid',
        gridTemplateColumns: 'auto minmax(0, 1fr) auto',
        alignItems: 'center',
        gap: 16,
        padding: '10px 14px',
        borderRadius: 16,
        background: '#fff',
        boxShadow: '0 1px 2px rgba(15,23,42,.05)',
      }}
    >
      <div className="row" style={{ gap: 12, minWidth: 0 }}>
        <Avatar nome={nome} cores={{ fundo: '#E3F0FA', texto: 'var(--blue)' }} />
        <div style={{ minWidth: 0, lineHeight: 1.3, whiteSpace: 'nowrap' }}>
          <div style={{ fontSize: 16, fontWeight: 650 }}>{nome}</div>
          <div className="t13" style={{ color: 'var(--ink2)' }}>
            <b className="num">{nascimento}</b>
            {complementoDoNascimento !== undefined && `, ${complementoDoNascimento}`}
          </div>
        </div>
      </div>
      <div className="row" style={{ gap: 10, minWidth: 0, whiteSpace: 'nowrap', overflow: 'hidden' }}>
        {alergia !== undefined && alergia !== '' && (
          <span className="al-chip" style={{ flex: 'none' }}>
            Alergia: {alergia}
          </span>
        )}
        {resumo !== undefined && (
          <span className="t13 clamp1" style={{ minWidth: 0, color: 'var(--ink2)' }} title={resumo}>
            {resumo}
          </span>
        )}
      </div>
      <div className="row" style={{ gap: 8 }}>
        {acoes}
      </div>
    </section>
  );
}
