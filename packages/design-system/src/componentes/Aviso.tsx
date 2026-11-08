import type { ReactNode } from 'react';
import { Icone, type NomeDoIcone } from './Icone';

/**
 * Três níveis de alerta (R4): bloqueante impede o ato, atenção pede ação hoje sem interromper,
 * informativo dá contexto. `feito` é a confirmação de que o servidor concluiu (nunca antes da resposta).
 */
export type NivelDoAviso = 'bloqueante' | 'atencao' | 'informativo' | 'feito';

const CONFIG: Record<NivelDoAviso, { classe: string; icone: NomeDoIcone; papel: 'alert' | 'status'; prefixo: string }> = {
  bloqueante: { classe: 'banner err', icone: 'erro', papel: 'alert', prefixo: 'Bloqueado' },
  atencao: { classe: 'banner off', icone: 'alerta', papel: 'status', prefixo: 'Atenção' },
  informativo: { classe: 'banner info', icone: 'info', papel: 'status', prefixo: 'Informação' },
  feito: { classe: 'banner ok', icone: 'confere', papel: 'status', prefixo: 'Feito' },
};

export interface PropsDoAviso {
  nivel: NivelDoAviso;
  /** O que aconteceu (frase curta, em negrito). */
  titulo?: string;
  /** O que já foi feito e o que fazer agora (R8). */
  children?: ReactNode;
  /** Código de suporte (`trace_id`), discreto ao final (R8, R14). */
  codigoDeSuporte?: string;
  acoes?: ReactNode;
}

export function Aviso({ nivel, titulo, children, codigoDeSuporte, acoes }: PropsDoAviso) {
  const c = CONFIG[nivel];
  return (
    <div className={c.classe} role={c.papel}>
      <Icone nome={c.icone} tamanho={20} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <span className="sr">{c.prefixo}: </span>
        <div className="t14">
          {titulo !== undefined && <b>{titulo} </b>}
          {children}
        </div>
        {codigoDeSuporte !== undefined && codigoDeSuporte !== '' && (
          <div className="suporte" style={{ marginTop: 6 }}>
            Código de suporte: {codigoDeSuporte}
          </div>
        )}
        {acoes !== undefined && (
          <div className="row wrap" style={{ gap: 8, marginTop: 10 }}>
            {acoes}
          </div>
        )}
      </div>
    </div>
  );
}
