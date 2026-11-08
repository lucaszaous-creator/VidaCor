import { Chip, formatarData, MenuDeContexto, useMenuDeContexto, type AcaoDoMenu, type CorDoChip } from '@vidacor/design-system';
import type { ObjetoDoMenu } from '@vidacor/api-client';
import type { ReactNode } from 'react';
import { MenuDoServidor } from './MenuDoServidor';

/**
 * Objetos sem rota de ações no servidor (pendências, estudos) abrem o menu vazio com o motivo. A tela
 * não inventa ações (docs/19 § 8).
 */
export const ACOES_DO_SERVIDOR: AcaoDoMenu[] = [];
export const MOTIVO_SEM_ACOES = 'Este item ainda não tem ações no menu';

export interface PropsDaLinhaDoPaciente {
  nome: string;
  /** `AAAA-MM-DD`. */
  nascimento: string;
  /** Hora ou código à esquerda. */
  inicio?: string;
  /** Exame ou complemento abaixo do nome. */
  detalhe?: string;
  situacao?: { texto: string; cor: CorDoChip };
  /** Marca curta de atenção (ex.: "Homônimo hoje"), sempre em texto. */
  marca?: ReactNode;
  selecionada: boolean;
  aoAbrir: () => void;
  /** Complemento do cabeçalho do menu (exame, horário). */
  contextoDoMenu?: string;
  /** Objeto cujas ações o servidor manda (agendamento, laudo). */
  menuDe?: { objeto: ObjetoDoMenu; id: string };
}

/** Linha de paciente em lista de painel, com os dois identificadores e menu de contexto. */
export function LinhaDoPaciente({ nome, nascimento, inicio, detalhe, situacao, marca, selecionada, aoAbrir, contextoDoMenu, menuDe }: PropsDaLinhaDoPaciente) {
  const menu = useMenuDeContexto();
  const nasc = formatarData(nascimento);
  const cabecalho = { titulo: nome, subtitulo: `Nascimento ${nasc}${contextoDoMenu === undefined ? '' : `, ${contextoDoMenu}`}` };
  return (
    <>
      <button
        type="button"
        className="lp"
        aria-current={selecionada}
        aria-label={[nome, `nascimento ${nasc}`, inicio, detalhe, situacao?.texto].filter(Boolean).join(', ')}
        onClick={aoAbrir}
        {...menu.propsDoAlvo}
      >
        {inicio !== undefined && (
          <span className="t14 num mute" style={{ width: 44, flex: 'none' }}>
            {inicio}
          </span>
        )}
        <span style={{ flex: 1, minWidth: 0 }}>
          <span className="t clamp1" style={{ display: 'block' }}>
            {nome}
          </span>
          <span className="s" style={{ display: 'block' }}>
            <span className="num">{nasc}</span>
            {detalhe !== undefined && `, ${detalhe}`}
          </span>
          {marca !== undefined && (
            <span className="t13" style={{ display: 'block', color: 'var(--amber)', fontWeight: 600 }}>
              {marca}
            </span>
          )}
        </span>
        {situacao !== undefined && <Chip cor={situacao.cor} texto={situacao.texto} />}
      </button>
      {menuDe === undefined ? (
        <MenuDeContexto estado={menu} cabecalho={cabecalho} acoes={ACOES_DO_SERVIDOR} motivoSemAcoes={MOTIVO_SEM_ACOES} aoEscolher={() => undefined} />
      ) : (
        <MenuDoServidor estado={menu} objeto={menuDe.objeto} id={menuDe.id} cabecalho={cabecalho} />
      )}
    </>
  );
}
