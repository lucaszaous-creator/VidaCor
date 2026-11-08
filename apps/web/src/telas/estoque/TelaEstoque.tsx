import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { ItemDeEstoque, RastreioDoLote } from '@vidacor/api-client';
import { Aviso, Botao, Campo, CampoDeData, Chip, formatarData, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const CHAVE = ['estoque', 'itens'] as const;

/**
 * Estoque com lote e validade (Estoque.dc.html). Os lotes aparecem na ordem de saída (vence antes,
 * sai antes); a baixa de contraste é feita pelo exame. A entrada é do estoque e da gerência; o
 * rastreio por lote (quem recebeu) é da gerência e fica registrado no lastro.
 */
export function TelaEstoque() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const perfis = sessao.data?.usuario.perfis ?? [];
  const itens = useQuery({ queryKey: CHAVE, queryFn: () => api.estoque.itens() });
  const [selecionado, setSelecionado] = useState<string | null>(null);
  const item = itens.data?.find((i) => i.id === selecionado) ?? null;
  return (
    <>
      <CabecalhoDaTela titulo="Estoque" complemento={<span className="t14 mute">Sai primeiro o lote que vence antes</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Itens" style={{ padding: 0 }}>
          <div className="scroll">
            <EstadoDaConsulta oQue="os itens do estoque" carregando={itens.isPending} erro={itens.error} aoTentarDeNovo={() => void itens.refetch()} />
            {itens.data?.length === 0 && <Vazio titulo="Nenhum item cadastrado." />}
            {itens.data?.map((i) => (
              <button
                key={i.id}
                type="button"
                className="lp"
                aria-current={i.id === selecionado}
                onClick={() => {
                  setSelecionado(i.id);
                }}
              >
                <span style={{ flex: 1, minWidth: 0 }}>
                  <span className="t clamp1" style={{ display: 'block' }}>
                    {i.nome}
                  </span>
                  <span className="s" style={{ display: 'block' }}>
                    {i.local}, {i.saldo_valido} {i.medida}(s) dentro da validade, mínimo {i.minimo}
                  </span>
                </span>
                {i.abaixo_do_minimo ? <Chip cor="rosa" texto="Abaixo do mínimo" /> : <Chip cor="verde" texto="Normal" />}
              </button>
            ))}
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do item">
          <div className="scroll col" style={{ gap: 14 }}>
            {item === null ? (
              <Vazio titulo="Escolha um item da lista." detalhe="Os lotes aparecem na ordem de saída." />
            ) : (
              <DetalheDoItem key={item.id} item={item} podeEntrada={perfis.some((p) => p === 'estoque' || p === 'gerente')} podeRastrear={perfis.includes('gerente')} />
            )}
          </div>
        </section>
      </div>
    </>
  );
}

function DetalheDoItem({ item, podeEntrada, podeRastrear }: { item: ItemDeEstoque; podeEntrada: boolean; podeRastrear: boolean }) {
  const { api } = useDependencias();
  const [rastreio, setRastreio] = useState<RastreioDoLote | null>(null);
  const [erroRastreio, setErroRastreio] = useState<unknown>(null);
  return (
    <>
      <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{item.nome}</h2>
      <h3 className="t15 b" style={{ margin: 0 }}>
        Lotes, na ordem de saída
      </h3>
      {item.lotes.length === 0 && <p className="t14 mute">Sem lote com saldo.</p>}
      {item.lotes.map((l) => (
        <div key={l.id} className="li" style={{ padding: '8px 0' }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div className="t14 b num">{l.numero}</div>
            <div className="t13 mute">Validade {formatarData(l.validade)}</div>
          </div>
          {l.vencido && <Chip cor="rosa" texto="Vencido: não sai" />}
          <span className="t14 num">
            {l.saldo} {item.medida}(s)
          </span>
          {podeRastrear && (
            <Botao
              pequeno
              onPress={() => {
                setErroRastreio(null);
                api.estoque.rastreio(l.id).then(setRastreio, setErroRastreio);
              }}
            >
              Rastrear lote
            </Botao>
          )}
        </div>
      ))}
      {erroRastreio !== null && <ErroDoAto erro={erroRastreio} ato="rastrear o lote" />}
      {rastreio !== null && (
        <Aviso nivel="informativo" titulo={`Lote ${rastreio.numero}: ${String(rastreio.pacientes.length)} paciente(s).`}>
          {rastreio.pacientes.length === 0
            ? 'Nenhum paciente recebeu este lote.'
            : rastreio.pacientes.map((p) => (
                <span key={`${p.atendimento_codigo}-${p.aplicado_em}`} style={{ display: 'block' }}>
                  {p.paciente.nome}, nascimento <span className="num">{formatarData(p.paciente.nascimento)}</span>, {p.atendimento_codigo},{' '}
                  {formatarDataHora(p.aplicado_em, FUSO_DA_CLINICA)}
                </span>
              ))}
        </Aviso>
      )}
      {podeEntrada && <EntradaDeLote item={item} />}
    </>
  );
}

function EntradaDeLote({ item }: { item: ItemDeEstoque }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [lote, setLote] = useState('');
  const [validade, setValidade] = useState<CalendarDate | null>(null);
  const [quantidade, setQuantidade] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (_: undefined, chave: string) =>
      api.estoque.entrada({ item_id: item.id, numero_lote: lote.trim(), validade: validade?.toString() ?? '', quantidade: Number(quantidade) }, { idempotencyKey: chave }),
    { aoConcluir: () => cliente.invalidateQueries({ queryKey: CHAVE }) },
  );
  const qtdOk = /^\d{1,6}$/.test(quantidade) && Number(quantidade) > 0;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Entrada de lote"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (lote.trim().length >= 2 && validade !== null && qtdOk) op.enviar(undefined);
      }}
    >
      <h3 className="t15 b" style={{ margin: 0 }}>
        Entrada de lote
      </h3>
      <div className="grid g2" style={{ gap: 10 }}>
        <Campo rotulo="Lote" value={lote} onChange={setLote} erro={tentou && lote.trim().length < 2 ? 'Digite o número do lote.' : null} />
        <CampoDeData rotulo="Validade" valor={validade} aoMudar={setValidade} erro={tentou && validade === null ? 'Digite a validade.' : null} />
        <Campo rotulo="Quantidade" value={quantidade} onChange={setQuantidade} inputMode="numeric" erro={tentou && !qtdOk ? 'Digite a quantidade.' : null} />
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="dar entrada no lote" />}
      {op.resultado !== null && (
        <Aviso nivel="feito" titulo={`Lote ${op.resultado.numero_lote} com ${String(op.resultado.saldo_do_lote)} em estoque.`}>
          O lote entrou na fila de saída pela validade.
        </Aviso>
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Dar entrada
        </Botao>
      </div>
    </form>
  );
}
