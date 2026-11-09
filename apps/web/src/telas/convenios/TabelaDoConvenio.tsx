import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { ConvenioDetalhe, NovaPropostaDeTabela, PropostaDeTabela } from '@vidacor/api-client';
import { Aviso, Botao, Busca, Campo, CampoDeData, Escolha, formatarDataHora, formatarDinheiro, formatarPontosBase, Tabela, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CHAVE_CONVENIOS, dataCurta, SITUACAO_DA_TABELA } from './comum';

function normalizar(t: string): string {
  return t
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLocaleLowerCase('pt-BR');
}

/** "5,20" vira 520 pontos base (só leitura do campo; quem confere o limite é o servidor). */
function lerPercentual(t: string): number | null {
  const m = /^(\d{1,2})(?:[,.](\d{1,2}))?$/.exec(t.trim());
  if (m === null) return null;
  return Number(m[1]) * 100 + Number((m[2] ?? '0').padEnd(2, '0'));
}

function Maiores({ p }: { p: { maiores: PropostaDeTabela['maiores'] } }) {
  return (
    <Tabela
      rotulo="Maiores mudanças"
      colunas={[
        { id: 'exame', rotulo: 'Maiores mudanças', cabecalhoDaLinha: true, celula: (m) => m.descricao },
        { id: 'antes', rotulo: 'Hoje', direita: true, semQuebra: true, celula: (m) => (typeof m.antes_centavos === 'number' ? formatarDinheiro(m.antes_centavos) : 'Novo') },
        { id: 'depois', rotulo: 'Depois', direita: true, semQuebra: true, celula: (m) => formatarDinheiro(m.depois_centavos) },
      ]}
      linhas={p.maiores.map((m) => ({ ...m, id: m.tuss }))}
    />
  );
}

/** Proposta aguardando decisão: a gerência que não propôs aprova ou recusa, com a senha de novo. */
function PropostaPendente({ c, p, gerencia, eu, aoDecidir }: { c: ConvenioDetalhe; p: PropostaDeTabela; gerencia: boolean; eu: string; aoDecidir: (r: PropostaDeTabela) => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const reautenticar = useReautenticar();
  const [motivo, setMotivo] = useState('');
  const [recusando, setRecusando] = useState(false);
  const op = useOperacao((aprovar: boolean, chave: string) => api.convenios.decidir(p.id, { aprovar, motivo: aprovar ? null : motivo.trim() }, { idempotencyKey: chave }), {
    aoConcluir: async (r) => {
      aoDecidir(r);
      await cliente.invalidateQueries({ queryKey: CHAVE_CONVENIOS });
    },
  });
  const propusQuem = p.proposta_por_id === eu;
  const oQue = p.tipo === 'reajuste' && typeof p.reajuste_pontos_base === 'number' ? `Reajuste de ${formatarPontosBase(p.reajuste_pontos_base)}` : 'Tabela com preços novos';
  return (
    <section className="col" aria-label="Proposta aguardando aprovação" style={{ gap: 10, border: '1px solid var(--line)', borderRadius: 14, padding: 14 }}>
      <div className="t15 b">
        {oQue} para {c.operadora}, a partir de {dataCurta(p.vigente_desde)}
      </div>
      <div className="t13 mute">
        Proposta por {p.proposta_por} em {formatarDataHora(p.proposta_em, FUSO_DA_CLINICA)}: {p.motivo}. Documento: {p.documento}. {p.itens} preços; os que já existiam somam{' '}
        <span className="num">{formatarDinheiro(p.total_antes_centavos)}</span> hoje e <span className="num">{formatarDinheiro(p.total_depois_centavos)}</span> depois.
      </div>
      <Maiores p={p} />
      {gerencia && propusQuem && <div className="t13">Quem propôs não decide: a aprovação é de outra pessoa da gerência.</div>}
      {gerencia && !propusQuem && (
        <>
          {recusando && <Campo rotulo="Motivo da recusa" value={motivo} onChange={setMotivo} />}
          {op.erro !== null && (
            <ErroDoAto
              erro={op.erro}
              ato="decidir a proposta"
              aoReautenticar={() => {
                reautenticar('decidir-tabela', op.chave(), {}, () => {
                  op.enviar(!recusando);
                });
              }}
            />
          )}
          <div className="row" style={{ gap: 8 }}>
            {!recusando && (
              <Botao
                variante="principal"
                ocupado={op.enviando}
                onPress={() => {
                  op.enviar(true);
                }}
              >
                Aprovar tabela
              </Botao>
            )}
            <Botao
              ocupado={op.enviando && recusando}
              isDisabled={recusando && motivo.trim().length < 5}
              onPress={() => {
                if (recusando) op.enviar(false);
                else setRecusando(true);
              }}
            >
              {recusando ? 'Confirmar recusa' : 'Recusar'}
            </Botao>
          </div>
          <div className="t13 mute">Pede sua senha de novo. Aprovada, a versão nova nasce com estes preços, calculados pelo servidor.</div>
        </>
      )}
      {!gerencia && <div className="t13 mute">Aguardando a aprovação de uma pessoa da gerência.</div>}
    </section>
  );
}

/** Reajuste em lote (faturamento): prévia calculada pelo servidor e proposta para a segunda pessoa. */
function PropostaDeReajuste({ c, aoFechar }: { c: ConvenioDetalhe; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [percentual, setPercentual] = useState('');
  const [desde, setDesde] = useState<CalendarDate | null>(null);
  const [descricao, setDescricao] = useState('');
  const [documento, setDocumento] = useState('');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const pb = lerPercentual(percentual);
  const previa = useQuery({
    queryKey: [...CHAVE_CONVENIOS, c.id, 'previa', pb],
    queryFn: () => api.convenios.previa(c.id, pb ?? 0),
    enabled: pb !== null && pb > 0,
    retry: false,
  });
  const op = useOperacao((n: NovaPropostaDeTabela, chave: string) => api.convenios.propor(c.id, n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: CHAVE_CONVENIOS }),
  });
  const erros = {
    percentual: pb === null || pb <= 0 ? 'Percentual como 5,20.' : null,
    desde: desde === null ? 'Diga quando começa.' : null,
    descricao: descricao.trim().length >= 3 ? null : 'Dê um nome à versão.',
    documento: documento.trim().length >= 3 ? null : 'Diga o documento (ex.: carta de reajuste).',
    motivo: motivo.trim().length >= 5 ? null : 'Diga o motivo (5 letras ou mais).',
  };
  if (op.resultado !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={`Proposta ${op.resultado.codigo} enviada para aprovação.`}
        acoes={
          <Botao pequeno onPress={aoFechar}>
            Fechar
          </Botao>
        }
      >
        A tabela vigente não muda. A versão nova só nasce quando outra pessoa da gerência aprovar.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Reajuste em lote"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (Object.values(erros).every((e) => e === null) && desde !== null && pb !== null)
          op.enviar({ vigente_desde: desde.toString(), descricao: descricao.trim(), documento: documento.trim(), motivo: motivo.trim(), reajuste_pontos_base: pb });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Reajuste em lote, {c.operadora}
      </h2>
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Campo rotulo="Percentual de reajuste" value={percentual} onChange={setPercentual} inputMode="decimal" placeholder="5,20" erro={tentou ? erros.percentual : null} />
        <CampoDeData rotulo="Vigência a partir de" valor={desde} aoMudar={setDesde} ajuda="A partir de amanhã" erro={tentou ? erros.desde : null} />
        <Campo rotulo="Nome da versão" value={descricao} onChange={setDescricao} placeholder="Unimed 2027" erro={tentou ? erros.descricao : null} />
        <Campo rotulo="Documento" value={documento} onChange={setDocumento} placeholder="Carta de reajuste 2027" erro={tentou ? erros.documento : null} />
      </div>
      {previa.data !== undefined && (
        <div className="col" style={{ gap: 8 }}>
          <div className="t13">
            Prévia calculada pelo servidor sobre a versão {previa.data.base_versao}: {previa.data.afetados} procedimentos, de{' '}
            <span className="num">{formatarDinheiro(previa.data.total_antes_centavos)}</span> para <span className="num">{formatarDinheiro(previa.data.total_depois_centavos)}</span> na soma da tabela.
          </div>
          <Maiores p={previa.data} />
        </div>
      )}
      {previa.error !== null && <ErroDoAto erro={previa.error} ato="calcular a prévia" />}
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou ? erros.motivo : null} />
      <div className="t13 mute">Quem pede o reajuste não pode aprovar: a gerência confere e confirma com a senha.</div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="propor o reajuste" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Enviar para segunda pessoa aprovar
        </Botao>
        <Botao onPress={aoFechar}>Cancelar</Botao>
      </div>
    </form>
  );
}

/** Aba "Tabela de preços": versão vigente (ou outra), proposta pendente e reajuste em lote. */
export function TabelaDoConvenio({ c, faturista, gerencia, eu }: { c: ConvenioDetalhe; faturista: boolean; gerencia: boolean; eu: string }) {
  const { api } = useDependencias();
  const [versao, setVersao] = useState('');
  const [busca, setBusca] = useState('');
  const [reajustando, setReajustando] = useState(false);
  const [decidida, setDecidida] = useState<PropostaDeTabela | null>(null);
  const precos = useQuery({
    queryKey: [...CHAVE_CONVENIOS, c.id, 'precos', versao],
    queryFn: () => api.convenios.precos(c.id, versao === '' ? undefined : Number(versao)),
    enabled: c.tabelas.length > 0,
  });
  if (reajustando) {
    return (
      <PropostaDeReajuste
        c={c}
        aoFechar={() => {
          setReajustando(false);
        }}
      />
    );
  }
  const t = precos.data;
  const linhas = (t?.itens ?? []).filter((i) => busca.trim() === '' || normalizar(`${i.tuss} ${i.descricao}`).includes(normalizar(busca.trim())));
  return (
    <div className="col" style={{ gap: 12 }}>
      {decidida !== null && (
        <Aviso nivel="feito" titulo={decidida.situacao === 'aprovada' ? `Tabela v${String(decidida.tabela_versao ?? '')} aprovada; começa em ${dataCurta(decidida.vigente_desde)}.` : 'Proposta recusada.'}>
          {decidida.situacao === 'aprovada' ? 'Até lá vale a tabela vigente; as guias são conferidas pela data do atendimento.' : 'A tabela não mudou. O faturamento pode propor de novo.'}
        </Aviso>
      )}
      {c.proposta_pendente !== null && c.proposta_pendente !== undefined && <PropostaPendente c={c} p={c.proposta_pendente} gerencia={gerencia} eu={eu} aoDecidir={setDecidida} />}
      <div className="row wrap" style={{ gap: 10, alignItems: 'end' }}>
        <div style={{ flex: '1 1 220px', minWidth: 0 }}>
          <Busca rotulo="Buscar na tabela" valor={busca} aoMudar={setBusca} placeholder="Código TUSS ou descrição" />
        </div>
        {c.tabelas.length > 0 && (
          <div style={{ flex: '0 1 260px', minWidth: 0 }}>
            <Escolha
              rotulo="Versão da tabela"
              valor={versao}
              aoMudar={setVersao}
              vazio="A vigente hoje"
              opcoes={c.tabelas.map((v) => ({ valor: String(v.versao), rotulo: `v${String(v.versao)}, desde ${dataCurta(v.vigente_desde)} (${SITUACAO_DA_TABELA[v.situacao]})` }))}
            />
          </div>
        )}
        {faturista && c.termos.ativo && (c.proposta_pendente === null || c.proposta_pendente === undefined) && c.tabelas.length > 0 && (
          <Botao
            pequeno
            variante="principal"
            onPress={() => {
              setReajustando(true);
            }}
          >
            Reajuste em lote
          </Botao>
        )}
      </div>
      {c.tabelas.length === 0 && <Vazio titulo="Sem tabela cadastrada." detalhe="Sem tabela, a auditoria bloqueia as guias deste convênio por falta de preço." />}
      <EstadoDaConsulta oQue="a tabela de preços" carregando={c.tabelas.length > 0 && precos.isPending} erro={precos.error} aoTentarDeNovo={() => void precos.refetch()} />
      {t !== undefined && (
        <>
          <Tabela
            rotulo={`Tabela versão ${String(t.versao)}`}
            vazio="Nenhum procedimento com esse código ou nome."
            colunas={[
              {
                id: 'exame',
                rotulo: 'TUSS e descrição',
                cabecalhoDaLinha: true,
                celula: (i) => (
                  <>
                    <span className="num t13 mute">{i.tuss}</span>
                    <div className="b">{i.descricao}</div>
                  </>
                ),
              },
              {
                id: 'valor',
                rotulo: 'Valor',
                direita: true,
                semQuebra: true,
                celula: (i) => (
                  <>
                    <span className="num">{formatarDinheiro(i.valor_centavos)}</span>
                    {typeof i.valor_anterior_centavos === 'number' && <div className="t12 mute num">antes {formatarDinheiro(i.valor_anterior_centavos)}</div>}
                  </>
                ),
              },
            ]}
            linhas={linhas.map((i) => ({ ...i, id: i.tuss }))}
          />
          <div className="t13 mute num">
            Tabela versão {t.versao}, {t.descricao}, {t.itens.length} procedimentos, desde {dataCurta(t.vigente_desde)}
          </div>
        </>
      )}
    </div>
  );
}
