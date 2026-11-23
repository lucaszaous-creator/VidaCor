// Lista à esquerda: exames abertos, com as séries de cada um, e os arquivos que ficaram de fora
// com o motivo (VIS-01, VIS-02). Rola dentro do painel.
import type { Estudo, Ignorado } from '../ponte';

export interface PropsDaLista {
  estudos: Estudo[];
  ignorados: Ignorado[];
  estudoAtual: number;
  serieAtual: number;
  aoEscolher: (estudo: number, serie: number) => void;
}

function quantidade(n: number, singular: string, plural: string): string {
  return `${String(n)} ${n === 1 ? singular : plural}`;
}

export function ListaDeSeries({ estudos, ignorados, estudoAtual, serieAtual, aoEscolher }: PropsDaLista) {
  const variosPacientes = new Set(estudos.map((e) => `${e.paciente.nome}|${e.paciente.nascimento ?? ''}`)).size > 1;
  return (
    <nav className="vz-lista" aria-label="Exames e séries">
      {variosPacientes && (
        <p className="vz-aviso-pacientes" role="status">
          Esta abertura tem exames de mais de um paciente. Confira o nome e o nascimento antes de exportar.
        </p>
      )}
      {estudos.map((e, ie) => (
        <section key={`${e.paciente.nome}-${String(ie)}`} className="vz-estudo" aria-label={`Exame ${e.descricao || 'sem descrição'} de ${e.paciente.nome}`}>
          <header className="vz-estudo-cab">
            <b>{e.paciente.nome || 'Paciente sem nome no exame'}</b>
            <span className="num">{e.paciente.nascimento ?? 'nascimento não informado'}</span>
            <span>
              {e.descricao || 'Exame sem descrição'}
              {e.data !== null && `, ${e.data}`}
            </span>
          </header>
          <ul className="vz-series">
            {e.series.map((s, is) => {
              const atual = ie === estudoAtual && is === serieAtual;
              const imagens = s.imagens.length;
              const quadros = s.imagens.reduce((t, i) => t + i.quadros, 0);
              return (
                <li key={`${String(s.numero)}-${String(is)}`}>
                  <button
                    type="button"
                    className={atual ? 'vz-serie atual' : 'vz-serie'}
                    aria-current={atual ? 'true' : undefined}
                    onClick={() => {
                      aoEscolher(ie, is);
                    }}
                  >
                    <span className="vz-mod">{s.modalidade || 'OT'}</span>
                    <span className="vz-serie-txt">
                      <span className="clamp1">{s.descricao || `Série ${String(s.numero ?? is + 1)}`}</span>
                      <span className="t13 mute">
                        {s.numero !== null && `Série ${String(s.numero)}, `}
                        {quantidade(imagens, 'imagem', 'imagens')}
                        {quadros > imagens && `, ${quantidade(quadros, 'quadro', 'quadros')}`}
                      </span>
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      ))}
      {ignorados.length > 0 && (
        <details className="vz-ignorados">
          <summary>{quantidade(ignorados.length, 'arquivo ficou de fora', 'arquivos ficaram de fora')}</summary>
          <ul>
            {ignorados.map((i, n) => (
              <li key={`${i.nome}-${String(n)}`}>
                <b>{i.nome}</b>: {i.motivo}
              </li>
            ))}
          </ul>
        </details>
      )}
    </nav>
  );
}
