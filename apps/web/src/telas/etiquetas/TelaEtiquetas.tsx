import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { DivergenciaDeSala, EstadoDaEtiqueta, LinhaDeEtiqueta, PedidoResolucaoDeSala } from '@vidacor/api-client';
import { Aviso, Botao, Chip, formatarData, formatarHora, Icone, Vazio, type CorDoChip } from '@vidacor/design-system';
import { useEffect, useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { AcoesDaEtiqueta } from './AcoesDaEtiqueta';

export const ESTADO_DA_ETIQUETA: Record<EstadoDaEtiqueta, { texto: string; cor: CorDoChip }> = {
  impressa: { texto: 'Impressa', cor: 'azul' },
  em_uso: { texto: 'Em uso', cor: 'verde' },
  invalidada: { texto: 'Invalidada', cor: 'ambar' },
  divergencia: { texto: 'Divergência', cor: 'rosa' },
};

const TIPO: Record<string, string> = { pulseira: 'Pulseira' };

interface AvisoDeResolucao {
  id: string;
  nome: string;
  descricao: string;
  etiqueta: string;
}

function AlertaDeDivergencia({ d, podeResolver, aoVer, aoResolver }: { d: DivergenciaDeSala; podeResolver: boolean; aoVer: () => void; aoResolver: (aviso: AvisoDeResolucao) => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  // O aviso do que foi feito fica com a tela: a divergência resolvida sai da lista ao recarregar.
  const op = useOperacao((p: PedidoResolucaoDeSala, chave: string) => api.etiquetas.resolverDivergencia(d.id, p, { idempotencyKey: chave }), {
    aoConcluir: async (r) => {
      aoResolver({ id: d.id, nome: d.paciente.nome, descricao: r.descricao, etiqueta: d.etiqueta });
      await cliente.invalidateQueries({ queryKey: ['etiquetas'] });
    },
  });
  return (
    <div className="col" style={{ gap: 6 }}>
      <Aviso
        nivel="bloqueante"
        titulo={`Exame bloqueado na ${d.sala_bipada}.`}
        acoes={
          <>
            <Botao pequeno onPress={aoVer}>
              Ver etiqueta
            </Botao>
            {podeResolver && (
              <Botao
                pequeno
                variante="principal"
                ocupado={op.enviando}
                onPress={() => {
                  op.enviar({ resolucao: 'encaminhado' });
                }}
              >
                Encaminhar à {d.sala_correta}
              </Botao>
            )}
          </>
        }
      >
        A pulseira de {d.paciente.nome} ({formatarData(d.paciente.nascimento)}, {d.exame}, {d.sala_correta}) foi bipada na {d.sala_bipada} às {formatarHora(d.aberta_em, FUSO_DA_CLINICA)}. O aparelho não recebeu a lista e o exame não começou.
      </Aviso>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="resolver a divergência" />}
    </div>
  );
}

function Lastro({ codigo, recepcao }: { codigo: string; recepcao: boolean }) {
  const { api } = useDependencias();
  const d = useQuery({ queryKey: ['etiquetas', 'detalhe', codigo], queryFn: () => api.etiquetas.detalhe(codigo) });
  if (d.data === undefined) return <EstadoDaConsulta oQue="o lastro da etiqueta" carregando={d.isPending} erro={d.error} aoTentarDeNovo={() => void d.refetch()} />;
  const { etiqueta: e, eventos } = d.data;
  const st = ESTADO_DA_ETIQUETA[e.estado];
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h2 className="t15 b num" style={{ margin: 0 }}>
          {e.codigo}
        </h2>
        <Chip cor={st.cor} texto={st.texto} />
      </div>
      <div style={{ border: '1px solid var(--line2)', borderRadius: 10, padding: '10px 12px' }}>
        <b style={{ display: 'block', textTransform: 'uppercase' }}>{e.paciente.nome}</b>
        <span className="t13 num" style={{ display: 'block' }}>
          Nasc. {formatarData(e.paciente.nascimento)}
        </span>
        <span className="t13 mute num" style={{ display: 'block' }}>
          {e.accession}, {e.exame}, {e.sala}
        </span>
      </div>
      {e.substitui !== null && e.substitui !== undefined && <p className="t13 mute" style={{ margin: 0 }}>Substitui a {e.substitui}.</p>}
      <ol className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }} aria-label="Eventos da etiqueta">
        {eventos.map((v, k) => (
          <li key={`${v.em}-${String(k)}`} className="row" style={{ gap: 10, alignItems: 'flex-start', padding: '8px 0', borderBottom: '1px solid var(--line)' }}>
            <span
              role="img"
              aria-label={v.nivel === 'alerta' ? 'Alerta' : 'Feito'}
              style={{ width: 24, height: 24, borderRadius: '50%', flex: 'none', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', background: v.nivel === 'alerta' ? 'var(--rose-bg)' : 'var(--green-bg)', color: v.nivel === 'alerta' ? 'var(--rose)' : 'var(--green)' }}
            >
              <Icone nome={v.nivel === 'alerta' ? 'alerta' : 'confere'} tamanho={14} />
            </span>
            <span style={{ flex: 1, minWidth: 0 }}>
              <span className="t14" style={{ display: 'block' }}>
                {v.descricao}
              </span>
              {v.por !== null && v.por !== undefined && <span className="t13 mute">{v.por}</span>}
            </span>
            <span className="t13 num mute">{formatarHora(v.em, FUSO_DA_CLINICA)}</span>
          </li>
        ))}
      </ol>
      <AcoesDaEtiqueta e={e} recepcao={recepcao} />
    </div>
  );
}

function Linha({ e, atual, aoEscolher }: { e: LinhaDeEtiqueta; atual: boolean; aoEscolher: () => void }) {
  const st = ESTADO_DA_ETIQUETA[e.estado];
  return (
    <tr className={atual ? 'sel' : undefined} aria-selected={atual}>
      <td className="nw">
        <button
          type="button"
          style={{ textAlign: 'left', border: 0, background: 'transparent', padding: 0, font: 'inherit', color: 'inherit', cursor: 'pointer', width: '100%' }}
          aria-label={`${e.codigo}, ${e.paciente.nome}, nascimento ${formatarData(e.paciente.nascimento)}`}
          onClick={aoEscolher}
        >
          <b className="num" style={{ display: 'block' }}>
            {e.codigo}
          </b>
          <span className="t13 mute">{TIPO[e.tipo] ?? e.tipo}</span>
        </button>
      </td>
      <td>
        <b style={{ display: 'block' }}>{e.paciente.nome}</b>
        <span className="t13 mute num">{formatarData(e.paciente.nascimento)}</span>
      </td>
      <td>
        <span className="t14" style={{ display: 'block' }}>
          {e.ultimo_evento?.descricao ?? 'Sem evento'}
        </span>
        <span className="t13 mute">
          {e.ultimo_evento === null || e.ultimo_evento === undefined ? '' : `${formatarHora(e.ultimo_evento.em, FUSO_DA_CLINICA)}, `}
          {e.exame}
        </span>
      </td>
      <td>
        <Chip cor={st.cor} texto={st.texto} />
      </td>
    </tr>
  );
}

/** Etiquetas e rastreio (Etiquetas.dc.html): o que foi impresso, bipado, reimpresso e bloqueado hoje. */
export function TelaEtiquetas() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const perfis = sessao.data?.usuario.perfis ?? [];
  const recepcao = perfis.includes('recepcao');
  const podeResolver = recepcao || perfis.includes('tecnico') || perfis.includes('gerente');
  const [texto, setTexto] = useState('');
  const [q, setQ] = useState('');
  const [escolhida, setEscolhida] = useState<string | null>(null);
  const [resolvidas, setResolvidas] = useState<AvisoDeResolucao[]>([]);
  useEffect(() => {
    const t = setTimeout(() => {
      const limpo = texto.trim();
      setQ(limpo.length >= 3 ? limpo : '');
    }, 300);
    return () => {
      clearTimeout(t);
    };
  }, [texto]);
  const p = useQuery({ queryKey: ['etiquetas', 'painel', q], queryFn: () => api.etiquetas.painel(q) });
  const d = p.data;
  const atual = d?.etiquetas.find((e) => e.codigo === escolhida) ?? d?.etiquetas[0] ?? null;
  return (
    <>
      <CabecalhoDaTela titulo="Etiquetas e rastreio">
        <input
          type="search"
          className="in"
          style={{ flex: '1 1 320px', maxWidth: 520 }}
          value={texto}
          placeholder="Bipe a etiqueta ou digite código ou nome"
          aria-label="Bipar etiqueta"
          onChange={(e) => {
            setTexto(e.target.value);
          }}
          onKeyDown={(e) => {
            // O leitor de código de barras termina com Enter: abre a etiqueta bipada na hora.
            if (e.key === 'Enter') {
              const codigo = texto.trim().toUpperCase();
              setQ(codigo);
              setEscolhida(codigo);
            }
          }}
        />
      </CabecalhoDaTela>
      {d !== undefined && (
        <div className="row wrap" style={{ gap: 24 }} aria-label="Resumo de hoje">
          {[
            [d.impressas, 'impressas hoje'],
            [d.bipadas, 'bipadas nas salas'],
            [d.reimpressoes, 'reimpressões com motivo'],
            [d.invalidadas, 'invalidadas'],
            [d.divergencias_abertas, d.divergencias_abertas === 1 ? 'divergência aberta' : 'divergências abertas'],
          ].map(([n, t]) => (
            <span key={String(t)} className="t14 mute">
              <b className="num" style={{ fontSize: 20, color: 'var(--ink)', marginRight: 6 }}>
                {n}
              </b>
              {t}
            </span>
          ))}
        </div>
      )}
      {resolvidas.map((r) => (
        <Aviso key={r.id} nivel="feito" titulo={`${r.nome}: divergência resolvida.`}>
          {r.descricao.charAt(0).toUpperCase() + r.descricao.slice(1)}. Registrado no lastro da etiqueta {r.etiqueta}.
        </Aviso>
      ))}
      {d?.divergencias.map((dv) => (
        <AlertaDeDivergencia
          key={dv.id}
          d={dv}
          podeResolver={podeResolver}
          aoResolver={(r) => {
            setResolvidas((atuais) => [...atuais.filter((x) => x.id !== r.id), r]);
          }}
          aoVer={() => {
            setTexto('');
            setQ('');
            setEscolhida(dv.etiqueta);
          }}
        />
      ))}
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.7fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Etiquetas de hoje" style={{ padding: 0 }}>
          <div className="scroll">
            <EstadoDaConsulta oQue="as etiquetas" carregando={p.isPending} erro={p.error} aoTentarDeNovo={() => void p.refetch()} />
            {d?.etiquetas.length === 0 ? (
              <div style={{ padding: 16 }}>
                <Vazio titulo="Nenhuma etiqueta encontrada." detalhe="Confira o código bipado ou busque pelo nome do paciente." />
              </div>
            ) : (
              <table className="tb sticky">
                <thead>
                  <tr>
                    <th className="nw">Etiqueta</th>
                    <th>Paciente</th>
                    <th>Último evento</th>
                    <th>Situação</th>
                  </tr>
                </thead>
                <tbody>
                  {d?.etiquetas.map((e) => (
                    <Linha
                      key={e.codigo}
                      e={e}
                      atual={atual?.codigo === e.codigo}
                      aoEscolher={() => {
                        setEscolhida(e.codigo);
                      }}
                    />
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </section>
        <aside className="card pane" aria-label="Lastro da etiqueta">
          <div className="scroll">{atual === null ? <Vazio titulo="Nenhuma etiqueta escolhida." /> : <Lastro key={atual.codigo} codigo={atual.codigo} recepcao={recepcao} />}</div>
        </aside>
      </div>
    </>
  );
}
