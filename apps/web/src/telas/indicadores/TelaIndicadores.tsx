import { useQuery } from '@tanstack/react-query';
import type { LaudoPorMedico, ProducaoPorExame } from '@vidacor/api-client';
import { Escolha, formatarDinheiro, Tabela } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const PERIODOS = [
  { valor: '0', rotulo: 'Hoje' },
  { valor: '6', rotulo: 'Últimos 7 dias' },
  { valor: '29', rotulo: 'Últimos 30 dias' },
  { valor: '89', rotulo: 'Últimos 90 dias' },
];
const DIAS = ['Dom', 'Seg', 'Ter', 'Qua', 'Qui', 'Sex', 'Sáb'];

function dia(deslocamento: number): string {
  const d = new Date(Date.now() - deslocamento * 86_400_000);
  return d.toLocaleDateString('sv-SE', { timeZone: FUSO_DA_CLINICA });
}

function horas(h: number | null | undefined): string {
  return typeof h === 'number' ? `${h.toLocaleString('pt-BR', { maximumFractionDigits: 1 })} h` : 'Sem laudo';
}

/**
 * Indicadores (Indicadores.dc.html, MVP do docs/25): produção, faltas, receita e tempo de laudo.
 * Tudo calculado no servidor e agregado: nenhum paciente aparece aqui. Glosa entra com o TISS.
 */
export function TelaIndicadores() {
  const { api } = useDependencias();
  const [periodo, setPeriodo] = useState('29');
  const de = dia(Number(periodo));
  const ate = dia(0);
  const q = useQuery({ queryKey: ['indicadores', de, ate], queryFn: () => api.indicadores.doPeriodo(de, ate) });
  const i = q.data;
  const maxOcupacao = Math.max(1, ...(i?.ocupacao.map((o) => o.agendamentos) ?? [1]));
  const horasDoDia = Array.from(new Set(i?.ocupacao.map((o) => o.hora) ?? [])).sort((a, b) => a - b);
  return (
    <>
      <CabecalhoDaTela titulo="Indicadores" complemento={<span className="t14 mute">Números agregados, sem paciente</span>}>
        <div style={{ width: 220 }}>
          <Escolha rotulo="Período" valor={periodo} aoMudar={setPeriodo} opcoes={PERIODOS} />
        </div>
      </CabecalhoDaTela>
      <div className="fill col" style={{ gap: 16, minHeight: 0 }}>
        <EstadoDaConsulta
          oQue="os indicadores"
          carregando={q.isPending}
          erro={q.error}
          aoTentarDeNovo={() => void q.refetch()}
          semPermissao={{ titulo: 'Você não vê indicadores', detalhe: 'Indicadores são da gerência e do financeiro.' }}
        />
        {i !== undefined && (
          <>
            <section className="grid" aria-label="Resumo" style={{ gridTemplateColumns: 'repeat(6, minmax(0, 1fr))', gap: 12 }}>
              {[
                ['Agendamentos', String(i.agendamentos)],
                ['Atendimentos', String(i.atendimentos)],
                ['Faltas', typeof i.taxa_de_faltas === 'number' ? `${String(i.faltas)} (${(i.taxa_de_faltas * 100).toLocaleString('pt-BR', { maximumFractionDigits: 1 })}%)` : String(i.faltas)],
                ['Receita (particular)', formatarDinheiro(i.receita_centavos)],
                ['Ticket médio', typeof i.ticket_medio_centavos === 'number' ? formatarDinheiro(i.ticket_medio_centavos) : 'Sem recebimento'],
                ['Tempo médio de laudo', horas(i.tempo_medio_de_laudo_horas)],
              ].map(([rotulo, valor]) => (
                <div key={rotulo} className="card" style={{ padding: 14 }}>
                  <div className="t13 mute">{rotulo}</div>
                  <div className="num" style={{ fontSize: 20, fontWeight: 650, whiteSpace: 'nowrap' }}>
                    {valor}
                  </div>
                </div>
              ))}
            </section>
            <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1.1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
              <section className="card pane" aria-label="Produção por exame" style={{ padding: 0 }}>
                <div className="scroll">
                  <Tabela<ProducaoPorExame & { id: string }>
                    rotulo="Produção por exame"
                    vazio="Sem atendimento no período."
                    linhas={i.producao.map((p) => ({ ...p, id: p.exame }))}
                    colunas={[
                      { id: 'exame', rotulo: 'Exame', cabecalhoDaLinha: true, celula: (p) => p.exame },
                      { id: 'n', rotulo: 'Atendimentos', direita: true, semQuebra: true, celula: (p) => p.atendimentos },
                    ]}
                  />
                </div>
              </section>
              <section className="card pane" aria-label="Laudo por médico" style={{ padding: 0 }}>
                <div className="scroll">
                  <Tabela<LaudoPorMedico & { id: string }>
                    rotulo="Laudo por médico"
                    vazio="Sem laudo assinado no período."
                    linhas={i.por_medico.map((m) => ({ ...m, id: m.medico }))}
                    colunas={[
                      { id: 'medico', rotulo: 'Médico', cabecalhoDaLinha: true, celula: (m) => m.medico },
                      { id: 'laudos', rotulo: 'Laudos', direita: true, semQuebra: true, celula: (m) => m.laudos },
                      {
                        id: 'horas',
                        rotulo: 'Laudo médio',
                        direita: true,
                        semQuebra: true,
                        celula: (m) => (typeof m.horas_medias === 'number' && m.horas_medias > 8 ? <b style={{ color: 'var(--rose)' }}>{horas(m.horas_medias)}, acima de 8 h</b> : horas(m.horas_medias)),
                      },
                    ]}
                  />
                </div>
              </section>
              <section className="card pane" aria-label="Ocupação por dia e hora">
                <div className="scroll">
                  {horasDoDia.length === 0 ? (
                    <p className="t14 mute">Sem agendamento no período.</p>
                  ) : (
                    <table className="tb" aria-label="Agendamentos por dia da semana e hora" style={{ fontSize: 12, whiteSpace: 'nowrap' }}>
                      <thead>
                        <tr>
                          <th style={{ padding: '6px 4px', whiteSpace: 'nowrap' }}>Hora</th>
                          {DIAS.map((d) => (
                            <th key={d} className="r" style={{ padding: '6px 4px', whiteSpace: 'nowrap' }}>
                              {d}
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {horasDoDia.map((h) => (
                          <tr key={h}>
                            <th className="num" style={{ padding: '6px 4px' }}>
                              {String(h).padStart(2, '0')}h
                            </th>
                            {DIAS.map((d, idx) => {
                              const n = i.ocupacao.find((o) => o.hora === h && o.dia_da_semana === idx)?.agendamentos ?? 0;
                              return (
                                <td key={d} className="r num" style={{ padding: '6px 4px', background: n === 0 ? undefined : `rgba(12, 114, 176, ${String(0.08 + (0.5 * n) / maxOcupacao)})` }}>
                                  {n === 0 ? '' : n}
                                </td>
                              );
                            })}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              </section>
            </div>
          </>
        )}
      </div>
    </>
  );
}
