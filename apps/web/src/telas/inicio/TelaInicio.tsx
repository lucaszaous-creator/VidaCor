import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { Chip, formatarData, formatarDinheiro, formatarHora, ListaDeDados, Vazio } from '@vidacor/design-system';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';

function Numero({ rotulo, valor, detalhe }: { rotulo: string; valor: number; detalhe?: string }) {
  return (
    <div className="card pane" style={{ padding: 16 }}>
      <div className="t13 mute">{rotulo}</div>
      <div className="num b" style={{ fontSize: 30, lineHeight: 1.2 }}>
        {valor}
      </div>
      {detalhe !== undefined && <div className="t13 mute">{detalhe}</div>}
    </div>
  );
}

/**
 * Início (Main.dc.html): o dia da unidade em números, os próximos pacientes e a ocupação por sala. O
 * caixa do dia aparece só para a gerência. Sem dado de estacionamento e satisfação, eles não aparecem.
 */
export function TelaInicio() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const d = useQuery({ queryKey: ['inicio'], queryFn: () => api.inicio(), refetchInterval: 60_000 });
  const primeiro = sessao.data?.usuario.nome.split(' ')[0] ?? '';
  if (d.data === undefined) {
    return (
      <>
        <CabecalhoDaTela titulo={primeiro === '' ? 'Início' : `Olá, ${primeiro}`} />
        <EstadoDaConsulta oQue="o resumo do dia" carregando={d.isPending} erro={d.error} aoTentarDeNovo={() => void d.refetch()} />
      </>
    );
  }
  const i = d.data;
  return (
    <>
      <CabecalhoDaTela
        titulo={primeiro === '' ? 'Início' : `Olá, ${primeiro}`}
        complemento={<span className="t14 mute">{formatarData(i.data)}</span>}
      >
        <Link to="/agendamentos/novo" className="btn btn-sm btn-p">
          Novo agendamento
        </Link>
      </CabecalhoDaTela>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', gap: 16 }}>
        <Numero rotulo="Exames agendados hoje" valor={i.exames_agendados} detalhe={`${String(i.realizados)} realizados, ${String(i.faltas)} faltas`} />
        <Numero rotulo="Aguardando agora" valor={i.aguardando_agora} detalhe="pacientes com check-in feito" />
        <Numero rotulo="Laudos para liberar" valor={i.laudos_a_liberar} detalhe={i.retificacoes_em_rascunho > 0 ? `${String(i.retificacoes_em_rascunho)} retificação em rascunho` : 'estudos conciliados sem assinatura'} />
        <Numero rotulo="Reservas sem guia" valor={i.reservas_sem_guia} detalhe="travam o faturamento do convênio" />
      </div>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.3fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16, marginTop: 16 }}>
        <section className="card pane" aria-label="Próximos pacientes">
          <div className="row" style={{ marginBottom: 8 }}>
            <h2 className="t15 b" style={{ margin: 0, flex: 1 }}>
              Próximos pacientes
            </h2>
            <Link to="/agenda" className="t14">
              Ver agenda
            </Link>
          </div>
          <div className="scroll">
            {i.proximos.length === 0 ? (
              <Vazio titulo="Ninguém mais agendado hoje." />
            ) : (
              <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }}>
                {i.proximos.map((p) => (
                  <li key={p.agendamento_id} className="row" style={{ gap: 10, padding: '8px 0', borderBottom: '1px solid var(--line)', alignItems: 'flex-start' }}>
                    <span className="num b" style={{ width: 52 }}>
                      {formatarHora(p.inicio, FUSO_DA_CLINICA)}
                    </span>
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <b style={{ display: 'block' }}>
                        {p.nome}, nascimento {formatarData(p.nascimento)}
                      </b>
                      <span className="t13 mute">
                        {p.exame}, {p.sala}
                      </span>
                    </span>
                    {p.situacao === 'reserva_provisoria' && <Chip cor="ambar" texto="Sem guia" />}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </section>
        <div className="col" style={{ gap: 16, minHeight: 0 }}>
          <section className="card pane" aria-label="Ocupação de hoje" style={{ flex: 1, minHeight: 0 }}>
            <h2 className="t15 b" style={{ margin: '0 0 4px' }}>
              Ocupação de hoje
            </h2>
            <p className="t13 mute" style={{ margin: '0 0 8px' }}>
              Horários marcados sobre a grade aberta (07:00 às 19:00).
            </p>
            <div className="scroll">
              <ul className="col" style={{ gap: 8, listStyle: 'none', margin: 0, padding: 0 }}>
                {i.ocupacao.map((o) => (
                  <li key={o.sala}>
                    <div className="row t14">
                      <span style={{ flex: 1 }}>{o.sala}</span>
                      <span className="num">{o.percentual}%</span>
                    </div>
                    <div role="progressbar" aria-label={`Ocupação da ${o.sala}`} aria-valuenow={o.percentual} aria-valuemin={0} aria-valuemax={100} style={{ height: 6, borderRadius: 3, background: 'var(--line)' }}>
                      <div style={{ width: `${String(o.percentual)}%`, height: 6, borderRadius: 3, background: 'var(--navy, #0A1A4A)' }} />
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          </section>
          {i.caixa !== null && i.caixa !== undefined && (
            <section className="card pane" aria-label="Caixa do dia">
              <h2 className="t15 b" style={{ margin: '0 0 8px' }}>
                Caixa do dia
              </h2>
              <ListaDeDados
                larguraDoRotulo={130}
                itens={[
                  { rotulo: 'Recebido', valor: <b className="num">{formatarDinheiro(i.caixa.recebido_centavos)}</b> },
                  { rotulo: 'Por forma', valor: <span className="num">PIX {formatarDinheiro(i.caixa.pix_centavos)}, cartão {formatarDinheiro(i.caixa.cartao_centavos)}, dinheiro {formatarDinheiro(i.caixa.dinheiro_centavos)}</span> },
                  { rotulo: 'Aguardando', valor: `${String(i.caixa.aguardando_pagamento)} pagamento(s)` },
                ]}
              />
            </section>
          )}
        </div>
      </div>
    </>
  );
}
