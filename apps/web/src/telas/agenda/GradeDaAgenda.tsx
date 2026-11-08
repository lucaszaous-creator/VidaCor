import type { ItemDaAgendaFase1 } from '@vidacor/api-client';
import { formatarData, formatarHora, useMenuDeContexto } from '@vidacor/design-system';
import { FUSO_DA_CLINICA, nomeCurto, rotuloDaHora, type Grade, type ItemPosicionado } from './grade';
import { MenuDoServidor } from '../../componentes/MenuDoServidor';
import { cabecalhoDoMenu } from './menu';
import { aparenciaDoAgendamento } from './situacoes';

interface PropsDoEvento {
  posicionado: ItemPosicionado;
  selecionado: boolean;
  aoAbrir: (item: ItemDaAgendaFase1) => void;
}

function EventoDaAgenda({ posicionado, selecionado, aoAbrir }: PropsDoEvento) {
  const { item, coluna, linha, extensao } = posicionado;
  const menu = useMenuDeContexto();
  const ap = aparenciaDoAgendamento(item.situacao);
  const hora = formatarHora(item.inicio, FUSO_DA_CLINICA);
  const sub = ap.classe === '' ? item.exame.nome : ap.texto;
  return (
    <>
      <button
        type="button"
        className={['ev', ap.classe, selecionado ? 'sel' : ''].filter(Boolean).join(' ')}
        style={{ gridColumn: coluna, gridRow: `${String(linha)} / span ${String(extensao)}` }}
        aria-label={`${item.paciente.nome}, nascimento ${formatarData(item.paciente.nascimento)}, ${hora}, ${item.exame.nome}, ${ap.texto}`}
        aria-pressed={selecionado}
        onClick={() => {
          aoAbrir(item);
        }}
        {...menu.propsDoAlvo}
      >
        <span className="n">
          {ap.ponto !== undefined && <i className="dot" style={{ background: ap.ponto }} aria-hidden="true" />}
          <span className="nm">{nomeCurto(item.paciente.nome)}</span>
        </span>
        <span className="s">{sub}</span>
      </button>
      <MenuDoServidor estado={menu} objeto="agendamento" id={item.id} cabecalho={cabecalhoDoMenu(item)} />
    </>
  );
}

export interface PropsDaGrade {
  grade: Grade;
  selecionado: string | null;
  aoAbrir: (item: ItemDaAgendaFase1) => void;
}

/** Agenda do dia por sala (Agenda.dc.html): uma coluna por sala, meia hora por linha. */
export function GradeDaAgenda({ grade, selecionado, aoAbrir }: PropsDaGrade) {
  const colunas = `56px repeat(${String(grade.colunas.length)}, minmax(0, 1fr))`;
  const linhas = `52px repeat(${String(grade.linhas)}, 46px)`;
  return (
    <div className="gd" style={{ gridTemplateColumns: colunas, gridTemplateRows: linhas }}>
      <div className="gh" style={{ gridTemplateColumns: colunas }}>
        <div />
        {grade.colunas.map((s) => (
          <div key={s.id}>
            <div className="t14 b clamp1" title={s.nome}>
              {s.nome}
            </div>
          </div>
        ))}
      </div>
      {Array.from({ length: grade.linhas }, (_, k) => {
        const r = k + 2;
        const min = grade.inicio + k * 30;
        return (
          <div key={r} style={{ display: 'contents' }}>
            <div style={{ gridColumn: '1 / -1', gridRow: r, borderTop: `1px solid ${k % 2 === 1 ? '#F4F5F7' : 'var(--line)'}` }} />
            <div className="hr" style={{ gridColumn: 1, gridRow: r }} aria-hidden="true">
              {k % 2 === 0 ? rotuloDaHora(min) : ''}
            </div>
          </div>
        );
      })}
      {grade.itens.map((p) => (
        <EventoDaAgenda key={p.item.id} posicionado={p} selecionado={selecionado === p.item.id} aoAbrir={aoAbrir} />
      ))}
    </div>
  );
}
