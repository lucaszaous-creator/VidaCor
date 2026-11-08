import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { ConjuntoDeRegras, GrupoDePendencias } from '@vidacor/api-client';
import { Abas, Chip, formatarData, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { chavesDoFaturamento } from './DetalheDaConta';
import { EFEITO } from './rotulos';
import { Conferir } from './TelaFaturamento';

type Selecao = { tipo: 'regra'; id: string } | { tipo: 'conjunto'; id: string } | null;

/**
 * Auditoria antes do faturamento (Auditoria-Faturamento.dc.html): guias conferidas, prontas, a
 * corrigir e bloqueadas; pendências agrupadas por regra com o que fazer; e as regras de cada
 * operadora com a versão. Cada guia guarda a versão da regra que a conferiu (docs/22 E5).
 */
export function TelaAuditoriaFaturamento() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const faturista = sessao.data?.usuario.perfis.includes('faturista') ?? false;
  const painel = useQuery({ queryKey: chavesDoFaturamento.auditoria, queryFn: () => api.faturamento.auditoria() });
  const [aba, setAba] = useState('pendencias');
  const [sel, setSel] = useState<Selecao>(null);
  const p = painel.data;
  const grupo = sel?.tipo === 'regra' ? (p?.grupos.find((g) => g.regra === sel.id) ?? null) : null;
  const conjunto = sel?.tipo === 'conjunto' ? (p?.conjuntos.find((c) => c.versao === sel.id) ?? null) : null;

  return (
    <>
      <CabecalhoDaTela titulo="Auditoria antes do faturamento" complemento={<span className="t14 mute">Cada guia guarda a versão da regra que a conferiu</span>}>
        {faturista && <Conferir />}
      </CabecalhoDaTela>
      {p !== undefined && (
        <section aria-label="Resumo da conferência" className="grid" style={{ gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', gap: 12 }}>
          <Numero rotulo="Guias conferidas" valor={p.resumo.conferidas} />
          <Numero rotulo="Prontas para enviar" valor={p.resumo.prontas} cor="var(--green)" />
          <Numero rotulo="Corrigir antes" valor={p.resumo.corrigir_antes} cor="var(--amber)" />
          <Numero rotulo="Bloqueadas" valor={p.resumo.bloqueadas} cor="var(--rose)" />
        </section>
      )}
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Pendências e regras">
          <div className="scroll">
            <EstadoDaConsulta
              oQue="a auditoria"
              carregando={painel.isPending}
              erro={painel.error}
              aoTentarDeNovo={() => void painel.refetch()}
              semPermissao={{ titulo: 'Você não vê a auditoria do faturamento', detalhe: 'A auditoria é do faturista e da gerência.' }}
            />
            {p !== undefined && (
              <Abas
                rotulo="Auditoria"
                selecionada={aba}
                aoSelecionar={(id) => {
                  setAba(id);
                  setSel(null);
                }}
                abas={[
                  {
                    id: 'pendencias',
                    rotulo: 'Pendências por regra',
                    conteudo: (
                      <div>
                        {p.grupos.length === 0 && <Vazio titulo="Nenhuma pendência aberta." detalhe="As guias conferidas entram no próximo lote de cada operadora." />}
                        {p.grupos.map((g) => (
                          <button
                            key={g.regra}
                            type="button"
                            className="lp"
                            aria-current={sel?.tipo === 'regra' && sel.id === g.regra}
                            onClick={() => {
                              setSel({ tipo: 'regra', id: g.regra });
                            }}
                          >
                            <span style={{ flex: 1, minWidth: 0 }}>
                              <span className="t" style={{ display: 'block' }}>
                                {g.titulo}
                              </span>
                              <span className="s" style={{ display: 'block' }}>
                                {g.contas.length} guia(s), {g.regra_versoes.join(' e ')}
                              </span>
                            </span>
                            <Chip cor={EFEITO[g.efeito].cor} texto={EFEITO[g.efeito].texto} />
                          </button>
                        ))}
                      </div>
                    ),
                  },
                  {
                    id: 'regras',
                    rotulo: 'Regras por operadora',
                    conteudo: (
                      <div>
                        {p.conjuntos.map((c) => (
                          <button
                            key={c.versao}
                            type="button"
                            className="lp"
                            aria-current={sel?.tipo === 'conjunto' && sel.id === c.versao}
                            onClick={() => {
                              setSel({ tipo: 'conjunto', id: c.versao });
                            }}
                          >
                            <span style={{ flex: 1, minWidth: 0 }}>
                              <span className="t" style={{ display: 'block' }}>
                                {c.nome}
                              </span>
                              <span className="s" style={{ display: 'block' }}>
                                {c.versao}, {c.regras.length} regra(s){c.operadoras.length === 0 ? ', sem contrato usando' : `, ${c.operadoras.join(', ')}`}
                              </span>
                            </span>
                          </button>
                        ))}
                      </div>
                    ),
                  },
                ]}
              />
            )}
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe da auditoria">
          <div className="scroll col" style={{ gap: 12 }}>
            {grupo !== null && <DetalheDoGrupo g={grupo} />}
            {conjunto !== null && <DetalheDoConjunto c={conjunto} />}
            {grupo === null && conjunto === null && <Vazio titulo="Escolha uma regra." detalhe="As regras vêm do manual da operadora, do padrão TISS e das glosas já recebidas." />}
          </div>
        </section>
      </div>
    </>
  );
}

function Numero({ rotulo, valor, cor }: { rotulo: string; valor: number; cor?: string }) {
  return (
    <div className="card" style={{ padding: '10px 16px' }}>
      <div className="t13 mute">{rotulo}</div>
      <div className="num" style={{ fontSize: 26, fontWeight: 650, color: cor ?? 'var(--ink)' }}>
        {valor}
      </div>
    </div>
  );
}

function DetalheDoGrupo({ g }: { g: GrupoDePendencias }) {
  return (
    <>
      <div className="row wrap" style={{ gap: 10 }}>
        <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0, flex: 1 }}>{g.titulo}</h2>
        <Chip cor={EFEITO[g.efeito].cor} texto={EFEITO[g.efeito].texto} />
      </div>
      <p className="t14" style={{ margin: 0 }}>
        {g.o_que_fazer}
      </p>
      <p className="t13 mute" style={{ margin: 0 }}>
        Origem: {g.origem}. Regra {g.regra_versoes.join(' e ')}.
      </p>
      <section aria-label="Guias com esta pendência" className="col" style={{ gap: 6 }}>
        {g.contas.map((c) => (
          <div key={c.conta_id} className="col" style={{ gap: 2, padding: '8px 12px', borderRadius: 'var(--r3)', background: 'var(--gray-bg)' }}>
            <b className="t14">
              {c.paciente.nome}, <span className="num">{formatarData(c.paciente.nascimento)}</span>
            </b>
            <span className="t13">
              {c.operadora}, guia {c.numero_guia_operadora} ({c.codigo})
            </span>
            <span className="t13 mute">{c.detalhe}</span>
            <Link className="t13" to="/faturamento" search={{ aba: 'guias', id: c.conta_id }}>
              Abrir a guia de {c.paciente.nome} no faturamento
            </Link>
          </div>
        ))}
      </section>
    </>
  );
}

function DetalheDoConjunto({ c }: { c: ConjuntoDeRegras }) {
  return (
    <>
      <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{c.nome}</h2>
      <p className="t14" style={{ margin: 0 }}>
        Versão {c.versao}, {c.regras.length} regra(s). {c.operadoras.length === 0 ? 'Nenhum contrato usa esta versão.' : `Usada por: ${c.operadoras.join(', ')}.`}
      </p>
      <section aria-label="Regras da versão" className="col" style={{ gap: 6 }}>
        {c.regras.map((r) => (
          <div key={r.regra} className="row wrap" style={{ gap: 10 }}>
            <span className="t14" style={{ flex: 1, minWidth: 0 }}>
              {r.titulo}
              <span className="t13 mute" style={{ display: 'block' }}>
                Origem: {r.origem}
              </span>
            </span>
            <Chip cor={EFEITO[r.efeito].cor} texto={EFEITO[r.efeito].texto} />
          </div>
        ))}
      </section>
      <p className="t13 mute" style={{ margin: 0 }}>
        Versão nova de regra entra por proposta revisada; a anterior continua valendo até a aprovação.
      </p>
    </>
  );
}
