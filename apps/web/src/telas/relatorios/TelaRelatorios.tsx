import { parseDate, type CalendarDate } from '@internationalized/date';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { IdDoRelatorio, PeriodoDeConsulta, RelatorioDoCatalogo } from '@vidacor/api-client';
import { Abas, Aviso, Botao, CampoDeData, Chip, formatarData, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { salvarNoNavegador } from '../../componentes/BaixarDocumento';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useReautenticar } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

function Previa({ id, periodo }: { id: IdDoRelatorio; periodo: PeriodoDeConsulta }) {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['relatorio', id, periodo.de, periodo.ate], queryFn: () => api.relatorios.previa(id, periodo) });
  if (q.data === undefined) {
    return <EstadoDaConsulta oQue="a prévia do relatório" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} />;
  }
  const p = q.data;
  const maior = Math.max(1, ...p.linhas.map((l) => l.valor));
  return (
    <div className="col" style={{ gap: 10, minHeight: 0 }}>
      <p className="t14 mute" style={{ margin: 0 }}>
        De {formatarData(p.de)} a {formatarData(p.ate)}, gerado às {formatarDataHora(p.gerado_em, FUSO_DA_CLINICA)}. {p.linhas.length} linha(s).
      </p>
      {p.dado_de_saude && (
        <Aviso nivel="informativo" titulo="Este relatório tem dado de saúde.">
          Na tela e no arquivo, o paciente aparece pelo código (PAC) e o exame pelo código TUSS.
        </Aviso>
      )}
      {p.linhas.length === 0 ? (
        <Vazio titulo="Nada no período escolhido." />
      ) : (
        <>
          <ul className="col" aria-label="Gráfico de barras" style={{ gap: 6, listStyle: 'none', margin: 0, padding: 0 }}>
            {p.linhas.slice(0, 8).map((l) => (
              <li key={`${l.a}-${l.b}`} style={{ display: 'grid', gridTemplateColumns: 'minmax(90px, 180px) minmax(0, 1fr) 90px', gap: 12, alignItems: 'center' }}>
                <span className="t14 clamp1" title={l.a}>
                  {l.a}
                </span>
                <span style={{ height: 14, borderRadius: 4, background: 'var(--line)' }} aria-hidden="true">
                  <span style={{ display: 'block', height: 14, borderRadius: 4, width: `${String(Math.round((l.valor / maior) * 100))}%`, background: 'var(--navy, #13285F)' }} />
                </span>
                <span className="num r">{l.c}</span>
              </li>
            ))}
          </ul>
          <table className="tb sticky" aria-label={`Prévia de ${p.titulo}`}>
            <thead>
              <tr>
                {p.colunas.map((c, i) => (
                  <th key={c} className={i === 2 ? 'r' : undefined}>
                    {c}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {p.linhas.map((l) => (
                <tr key={`${l.a}-${l.b}`}>
                  <td>{l.a}</td>
                  <td>{l.b}</td>
                  <td className="num r">{l.c}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}

function Exportar({ r, periodo }: { r: RelatorioDoCatalogo; periodo: PeriodoDeConsulta }) {
  const { api, salvarArquivo } = useDependencias();
  const cliente = useQueryClient();
  const reautenticar = useReautenticar();
  const m = useMutation({
    mutationFn: () => api.relatorios.exportar(r.id, periodo),
    onSuccess: async (doc) => {
      (salvarArquivo ?? salvarNoNavegador)(doc);
      await cliente.invalidateQueries({ queryKey: ['exportacoes'] });
    },
  });
  return (
    <div className="col" style={{ gap: 6 }}>
      <div className="row wrap" style={{ gap: 10 }}>
        <Botao
          variante="principal"
          ocupado={m.isPending}
          onPress={() => {
            m.mutate();
          }}
        >
          Exportar CSV
        </Botao>
        <span className="t13 mute">Pede senha e código do autenticador e fica no lastro. O arquivo não fica guardado no sistema.</span>
      </div>
      {m.isSuccess && (
        <p className="t13 mute num" role="status" style={{ margin: 0, overflowWrap: 'anywhere' }}>
          Arquivo gerado: {m.data.nomeDoArquivo}. Registrado no lastro como {m.data.codigoDaEmissao}, SHA-256 {m.data.sha256}.
        </p>
      )}
      {m.isError && (
        <ErroDoAto
          erro={m.error}
          ato="exportar o relatório"
          aoReautenticar={() => {
            reautenticar('exportar-relatorio', crypto.randomUUID(), {}, () => {
              m.mutate();
            });
          }}
          aoTentarDeNovo={() => {
            m.mutate();
          }}
        />
      )}
    </div>
  );
}

function Exportacoes() {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['exportacoes'], queryFn: () => api.relatorios.exportacoes() });
  if (q.data === undefined) {
    return <EstadoDaConsulta oQue="as exportações feitas" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} />;
  }
  if (q.data.length === 0) {
    return <Vazio titulo="Nenhuma exportação nos últimos 90 dias." />;
  }
  return (
    <table className="tb sticky" aria-label="Exportações feitas">
      <thead>
        <tr>
          <th>Arquivo</th>
          <th>Quem</th>
          <th>Quando</th>
          <th>Conteúdo</th>
        </tr>
      </thead>
      <tbody>
        {q.data.map((e) => (
          <tr key={e.codigo}>
            <td>
              <span style={{ display: 'block' }}>
                {e.titulo}, {e.formato.toUpperCase()}
              </span>
              <span className="t13 mute num" style={{ overflowWrap: 'anywhere' }}>
                {e.codigo}, {formatarData(e.de)} a {formatarData(e.ate)}
              </span>
            </td>
            <td>{e.quem}</td>
            <td className="nw">{formatarDataHora(e.quando, FUSO_DA_CLINICA)}</td>
            <td>
              <Chip cor={e.conteudo === 'pseudonimizado' ? 'ambar' : 'cinza'} texto={e.conteudo === 'pseudonimizado' ? 'Dado de saúde removido' : 'Sem dado de saúde'} />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function primeiroDoMes(hoje: CalendarDate): CalendarDate {
  return hoje.set({ day: 1 });
}

/**
 * Relatórios e exportação (Relatorios.dc.html): catálogo calculado no núcleo, prévia e exportação em
 * CSV com reautenticação. O servidor decide quais relatórios o perfil vê; dado de saúde sai como código.
 */
export function TelaRelatorios() {
  const { api } = useDependencias();
  const catalogo = useQuery({ queryKey: ['relatorios'], queryFn: () => api.relatorios.catalogo() });
  const [escolhido, setEscolhido] = useState<IdDoRelatorio | null>(null);
  const hoje = parseDate(new Date().toLocaleDateString('sv-SE', { timeZone: FUSO_DA_CLINICA }));
  const [de, setDe] = useState<CalendarDate | null>(() => primeiroDoMes(hoje));
  const [ate, setAte] = useState<CalendarDate | null>(hoje);
  if (catalogo.data === undefined) {
    return (
      <>
        <CabecalhoDaTela titulo="Relatórios" />
        <EstadoDaConsulta
          oQue="os relatórios"
          carregando={catalogo.isPending}
          erro={catalogo.error}
          aoTentarDeNovo={() => void catalogo.refetch()}
          semPermissao={{ titulo: 'Seu perfil não tem relatórios.', detalhe: 'Peça à coordenação se precisar de algum número.' }}
        />
      </>
    );
  }
  const lista = catalogo.data;
  const atual = lista.find((r) => r.id === escolhido) ?? lista[0];
  const grupos = [...new Set(lista.map((r) => r.grupo))];
  const periodoInvalido = de === null || ate === null || ate.compare(de) < 0 || ate.compare(de.add({ days: 365 })) > 0;
  const periodo: PeriodoDeConsulta = { de: de?.toString() ?? '', ate: ate?.toString() ?? '' };
  return (
    <>
      <CabecalhoDaTela titulo="Relatórios" complemento={<span className="t14 mute">Toda exportação pede sua senha de novo e fica no lastro.</span>} />
      {atual === undefined ? (
        <Vazio titulo="Seu perfil não tem relatórios no catálogo." />
      ) : (
        <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(220px, 280px) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
          <nav className="card pane" aria-label="Catálogo de relatórios" style={{ overflowY: 'auto' }}>
            {grupos.map((g) => (
              <div key={g}>
                <div className="t13 b mute" style={{ padding: '10px 0 4px' }}>
                  {g}
                </div>
                <ul className="col" style={{ gap: 4, listStyle: 'none', margin: 0, padding: 0 }}>
                  {lista
                    .filter((r) => r.grupo === g)
                    .map((r) => (
                      <li key={r.id}>
                        <button
                          type="button"
                          className="btn btn-sm"
                          aria-current={r.id === atual.id ? 'true' : undefined}
                          style={{ width: '100%', justifyContent: 'flex-start', textAlign: 'left', fontWeight: r.id === atual.id ? 650 : 400 }}
                          onClick={() => {
                            setEscolhido(r.id);
                          }}
                        >
                          {r.titulo}
                        </button>
                      </li>
                    ))}
                </ul>
              </div>
            ))}
          </nav>
          <section className="card pane" aria-label={atual.titulo} style={{ minHeight: 0 }}>
            <h2 className="t15 b" style={{ margin: 0 }}>
              {atual.titulo}
            </h2>
            <p className="t14 mute" style={{ margin: '2px 0 8px' }}>
              {atual.descricao}
            </p>
            <div className="row wrap" style={{ gap: 10, alignItems: 'flex-end' }}>
              <div style={{ width: 180 }}>
                <CampoDeData rotulo="De" valor={de} aoMudar={setDe} />
              </div>
              <div style={{ width: 180 }}>
                <CampoDeData rotulo="Até" valor={ate} aoMudar={setAte} erro={periodoInvalido ? 'Período de 1 a 366 dias, com o início antes do fim.' : null} />
              </div>
            </div>
            <div style={{ overflowY: 'auto', minHeight: 0, flex: 1, marginTop: 10 }}>
              <Abas
                rotulo="Relatório"
                abas={[
                  {
                    id: 'previa',
                    rotulo: 'Prévia e exportação',
                    conteudo: periodoInvalido ? (
                      <Vazio titulo="Ajuste o período para ver a prévia." />
                    ) : (
                      <div className="col" style={{ gap: 12 }}>
                        <Exportar r={atual} periodo={periodo} />
                        <Previa id={atual.id} periodo={periodo} />
                      </div>
                    ),
                  },
                  { id: 'feitas', rotulo: 'Exportações feitas', conteudo: <Exportacoes /> },
                ]}
              />
            </div>
          </section>
        </div>
      )}
    </>
  );
}
