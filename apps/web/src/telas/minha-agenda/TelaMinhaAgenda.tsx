import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { AusenciaDoMedico, GrupoDaMinhaAgenda, ItemDaMinhaAgenda, MinhaAgenda } from '@vidacor/api-client';
import { Abas, Botao, Chip, formatarData, formatarDiaPorExtenso, formatarHora, useMenuDeContexto, Vazio, type CorDoChip } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { MenuDoServidor } from '../../componentes/MenuDoServidor';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { NOME_DO_MOTIVO, PedidoDeBloqueio } from './PedidoDeBloqueio';

const GRUPOS: { id: GrupoDaMinhaAgenda | 'todos'; rotulo: string }[] = [
  { id: 'todos', rotulo: 'Todos' },
  { id: 'a_chegar', rotulo: 'A chegar' },
  { id: 'esperando', rotulo: 'Esperando' },
  { id: 'em_exame', rotulo: 'Em exame' },
  { id: 'liberado', rotulo: 'Liberados' },
];

const SITUACAO: Record<GrupoDaMinhaAgenda, { texto: string; cor: CorDoChip }> = {
  a_chegar: { texto: 'A chegar', cor: 'cinza' },
  esperando: { texto: 'Esperando', cor: 'rosa' },
  em_exame: { texto: 'Em exame', cor: 'violeta' },
  liberado: { texto: 'Liberado', cor: 'verde' },
  faltou: { texto: 'Faltou', cor: 'ambar' },
};

function minutosDesde(instante: string): number {
  return Math.max(0, Math.round((Date.now() - new Date(instante).getTime()) / 60_000));
}

function Numero({ rotulo, valor, detalhe }: { rotulo: string; valor: number; detalhe: string }) {
  return (
    <div style={{ padding: '4px 16px', borderLeft: '1px solid var(--line)', minWidth: 0 }}>
      <div className="t13 mute">{rotulo}</div>
      <div className="num b" style={{ fontSize: 26, lineHeight: 1.15 }}>
        {valor}
      </div>
      <div className="t13 mute">{detalhe}</div>
    </div>
  );
}

function situacaoDoItem(i: ItemDaMinhaAgenda): string {
  const s = SITUACAO[i.grupo].texto;
  if (i.grupo === 'esperando' && typeof i.esperando_desde === 'string') {
    return `${s} ${String(minutosDesde(i.esperando_desde))} min`;
  }
  if (i.grupo === 'liberado') {
    return i.laudo_situacao === 'assinado' ? 'Laudo assinado' : 'Laudo pendente';
  }
  return s;
}

function LinhaDoPaciente({ i }: { i: ItemDaMinhaAgenda }) {
  const menu = useMenuDeContexto();
  const detalhe = i.grupo === 'esperando' && typeof i.senha === 'string' ? `${i.sala}, senha ${i.senha}` : i.sala;
  return (
    <tr {...menu.propsDoAlvo} tabIndex={0} aria-label={`${i.paciente.nome}, nascimento ${formatarData(i.paciente.nascimento)}, ${i.exame}`}>
      <td className="num nw">{formatarHora(i.inicio, FUSO_DA_CLINICA)}</td>
      <td>
        <b style={{ display: 'block' }}>{i.paciente.nome}</b>
        <span className="t13 mute">nascimento {formatarData(i.paciente.nascimento)}</span>
      </td>
      <td>
        <span style={{ display: 'block' }}>{i.exame}</span>
        <span className="t13 mute">{detalhe}</span>
      </td>
      <td>
        <Chip cor={i.grupo === 'liberado' && i.laudo_situacao !== 'assinado' ? 'ambar' : SITUACAO[i.grupo].cor} texto={situacaoDoItem(i)} />
      </td>
      <td className="nw">
        {typeof i.laudo_id === 'string' && (
          <Link to="/laudos/$laudoId" params={{ laudoId: i.laudo_id }} className="t14">
            {i.laudo_situacao === 'assinado' ? 'Ver laudo' : 'Laudar'}
          </Link>
        )}
        <MenuDoServidor
          estado={menu}
          objeto="agendamento"
          id={i.agendamento_id}
          cabecalho={{ titulo: i.paciente.nome, subtitulo: `Nascimento ${formatarData(i.paciente.nascimento)}, ${i.exame}` }}
        />
      </td>
    </tr>
  );
}

function Pacientes({ a }: { a: MinhaAgenda }) {
  const [grupo, setGrupo] = useState<string>('todos');
  const contar = (g: string) => (g === 'todos' ? a.itens.length : a.itens.filter((i) => i.grupo === g).length);
  const visiveis = a.itens.filter((i) => grupo === 'todos' || i.grupo === grupo);
  return (
    <section className="card pane fill" aria-label="Pacientes do dia" style={{ minHeight: 0 }}>
      <Abas
        rotulo="Pacientes por situação"
        selecionada={grupo}
        aoSelecionar={setGrupo}
        abas={GRUPOS.map((g) => ({ id: g.id, rotulo: `${g.rotulo} ${String(contar(g.id))}`, conteudo: null }))}
      />
      <div className="scroll" style={{ overflowY: 'auto', minHeight: 0, marginTop: 8 }}>
        {visiveis.length === 0 ? (
          <Vazio titulo="Nenhum paciente nesta situação agora." />
        ) : (
          <table className="tb sticky" aria-label="Pacientes do dia">
            <thead>
              <tr>
                <th>Hora</th>
                <th>Paciente</th>
                <th>Exame</th>
                <th>Situação</th>
                <th>Ação</th>
              </tr>
            </thead>
            <tbody>
              {visiveis.map((i) => (
                <LinhaDoPaciente key={i.agendamento_id} i={i} />
              ))}
            </tbody>
          </table>
        )}
      </div>
      <p className="t13 mute" style={{ margin: '8px 0 0' }}>
        Você vê o prontuário completo só de quem tem vínculo de atendimento com você.
      </p>
    </section>
  );
}

function MeusPedidos() {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['ausencias'], queryFn: () => api.ausencias.listar() });
  const cancelar = useOperacao((id: string, chave: string) => api.ausencias.cancelar(id, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      await q.refetch();
    },
  });
  if (q.data === undefined) {
    return <EstadoDaConsulta oQue="os seus pedidos de bloqueio" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} linhas={2} />;
  }
  if (q.data.length === 0) {
    return <Vazio titulo="Nenhum pedido de bloqueio." />;
  }
  return (
    <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }}>
      {q.data.map((p: AusenciaDoMedico) => (
        <li key={p.id} style={{ padding: '8px 0', borderBottom: '1px solid var(--line)' }}>
          <b style={{ display: 'block' }}>
            {formatarData(p.inicio.slice(0, 10))} a {formatarData(new Date(new Date(p.fim).getTime() - 1).toISOString().slice(0, 10))}
          </b>
          <span className="t13 mute">
            {NOME_DO_MOTIVO[p.motivo]}, {p.codigo}.{' '}
            {p.situacao === 'aguardando'
              ? 'Aguardando a coordenação.'
              : p.situacao === 'aprovada'
                ? `Aprovado por ${p.decidido_por ?? 'coordenação'}.`
                : p.situacao === 'recusada'
                  ? `Recusado: ${p.motivo_decisao ?? ''}`
                  : 'Cancelado por você.'}
          </span>
          {p.situacao === 'aguardando' && (
            <div className="row" style={{ marginTop: 4 }}>
              <Botao
                pequeno
                ocupado={cancelar.enviando}
                onPress={() => {
                  cancelar.reiniciar();
                  cancelar.enviar(p.id);
                }}
              >
                Desistir do pedido
              </Botao>
            </div>
          )}
        </li>
      ))}
    </ul>
  );
}

function Lado({ a }: { a: MinhaAgenda }) {
  const maior = Math.max(1, ...a.semana.map((d) => d.pacientes));
  return (
    <section className="card pane" aria-label="Avisos e semana" style={{ minHeight: 0 }}>
      <Abas
        rotulo="Avisos e semana"
        abas={[
          {
            id: 'avisos',
            rotulo: `Avisos para mim ${String(a.avisos.length)}`,
            conteudo:
              a.avisos.length === 0 ? (
                <Vazio titulo="Nenhum aviso agora." />
              ) : (
                <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }}>
                  {a.avisos.map((v) => (
                    <li key={`${v.tipo}-${v.quando}-${v.texto}`} style={{ padding: '8px 0', borderBottom: '1px solid var(--line)' }}>
                      <b style={{ display: 'block' }}>{v.texto}</b>
                      <span className="t13 mute">
                        {v.tipo === 'laudo_vence' ? 'Prazo' : 'Recebidas'} {formatarHora(v.quando, FUSO_DA_CLINICA)}
                      </span>
                    </li>
                  ))}
                </ul>
              ),
          },
          {
            id: 'semana',
            rotulo: 'Minha semana',
            conteudo: (
              <ul className="col" style={{ gap: 8, listStyle: 'none', margin: 0, padding: 0 }}>
                {a.semana.map((d) => (
                  <li key={d.data}>
                    <div className="row t14">
                      <span style={{ flex: 1, fontWeight: d.data === a.data ? 650 : 400 }}>{formatarDiaPorExtenso(d.data)}</span>
                      <span className="num">{typeof d.ausencia === 'string' ? `${d.ausencia}, bloqueado` : d.pacientes === 0 ? 'Sem agenda' : `${String(d.pacientes)} pacientes`}</span>
                    </div>
                    <div style={{ height: 6, borderRadius: 3, background: 'var(--line)' }} aria-hidden="true">
                      <div style={{ width: `${String(Math.round((d.pacientes / maior) * 100))}%`, height: 6, borderRadius: 3, background: 'var(--navy, #0A1A4A)' }} />
                    </div>
                  </li>
                ))}
              </ul>
            ),
          },
          { id: 'pedidos', rotulo: 'Meus bloqueios', conteudo: <MeusPedidos /> },
        ]}
      />
    </section>
  );
}

/**
 * Minha agenda (Agenda-Medico.dc.html): o dia do médico com os pacientes agendados com ele, em grupos,
 * os laudos para assinar, avisos, a semana e o pedido de bloqueio da agenda. Quem decide o que ele vê
 * é o servidor; os dois identificadores aparecem em cada linha.
 */
export function TelaMinhaAgenda() {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['minha-agenda'], queryFn: () => api.minhaAgenda.obter(), refetchInterval: 30_000 });
  const [bloqueio, setBloqueio] = useState(false);
  if (q.data === undefined) {
    return (
      <>
        <CabecalhoDaTela titulo="Minha agenda" />
        <EstadoDaConsulta
          oQue="a sua agenda"
          carregando={q.isPending}
          erro={q.error}
          aoTentarDeNovo={() => void q.refetch()}
          semPermissao={{ titulo: 'Esta agenda é do médico executante.', detalhe: 'Use Agenda e check-in para ver a agenda da unidade.' }}
        />
      </>
    );
  }
  const a = q.data;
  const turnos = a.turnos.map((t) => `${t.sala} das ${formatarHora(t.inicio, FUSO_DA_CLINICA)} às ${formatarHora(t.fim, FUSO_DA_CLINICA)}`).join('; ');
  const primeiroEsperando = a.itens.find((i) => i.grupo === 'esperando');
  return (
    <>
      <CabecalhoDaTela
        titulo="Minha agenda"
        complemento={
          <span className="t14 mute">
            {formatarDiaPorExtenso(a.data)}.{turnos === '' ? ' Sem pacientes agendados com você hoje.' : ` ${turnos}.`}
          </span>
        }
      >
        <Botao
          pequeno
          onPress={() => {
            setBloqueio(true);
          }}
        >
          Bloquear horário
        </Botao>
      </CabecalhoDaTela>
      <div className="card" style={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', padding: '8px 0', marginBottom: 12 }}>
        <Numero rotulo="Pacientes hoje" valor={a.pacientes} detalhe={a.turnos.map((t) => t.sala).join(', ') || 'nenhum'} />
        <Numero rotulo="Já atendidos" valor={a.ja_atendidos} detalhe={`${String(a.atendidos_com_laudo_pendente)} com laudo pendente`} />
        <Numero
          rotulo="Esperando por você"
          valor={a.esperando}
          detalhe={primeiroEsperando === undefined ? 'ninguém agora' : `${primeiroEsperando.paciente.nome}, ${String(minutosDesde(primeiroEsperando.esperando_desde ?? new Date().toISOString()))} min`}
        />
        <Numero rotulo="Laudos para assinar" valor={a.laudos_para_assinar} detalhe={`${String(a.laudos_vencem_hoje)} vencem hoje`} />
      </div>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(280px, 340px)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <Pacientes a={a} />
        <Lado a={a} />
      </div>
      {bloqueio && (
        <PedidoDeBloqueio
          hoje={a.data}
          aoFechar={() => {
            setBloqueio(false);
          }}
        />
      )}
    </>
  );
}
