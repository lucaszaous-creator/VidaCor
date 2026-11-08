import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { Titulo } from '@vidacor/api-client';
import { Abas, Botao, Chip, Escolha, formatarData, formatarDinheiro, Tabela } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar } from '../../reautenticacao';
import { NovoLancamento } from './NovoLancamento';
import { Repasse } from './Repasse';

/**
 * Financeiro (Financeiro.dc.html, MVP do docs/25). Títulos a pagar e a receber com aprovação por outra
 * pessoa (duas acima de R$ 10 mil) e baixa única; repasse pela regra versionada. Tudo validado e
 * calculado no servidor.
 */
export function TelaFinanceiro() {
  const { api } = useDependencias();
  const resumo = useQuery({ queryKey: ['financeiro', 'resumo'], queryFn: () => api.financeiro.resumo() });
  const [aba, setAba] = useState('pagar');
  const r = resumo.data;
  return (
    <>
      <CabecalhoDaTela titulo="Financeiro" complemento={<span className="t14 mute">Acima de R$ 10 mil, duas aprovações</span>} />
      <EstadoDaConsulta
        oQue="o financeiro"
        carregando={resumo.isPending}
        erro={resumo.error}
        aoTentarDeNovo={() => void resumo.refetch()}
        semPermissao={{ titulo: 'Você não acessa o financeiro', detalhe: 'O financeiro é do setor financeiro e da gerência.' }}
      />
      {r !== undefined && (
        <>
          <section className="grid" aria-label="Resumo do mês" style={{ gridTemplateColumns: 'repeat(5, minmax(0, 1fr))', gap: 12 }}>
            {[
              ['Entradas do mês', formatarDinheiro(r.entradas_do_mes_centavos)],
              ['Saídas do mês', formatarDinheiro(r.saidas_do_mes_centavos)],
              ['Saldo em contas', formatarDinheiro(r.saldo_em_contas_centavos)],
              ['A pagar em aberto', formatarDinheiro(r.a_pagar_aberto_centavos)],
              ['A receber em aberto', formatarDinheiro(r.a_receber_aberto_centavos)],
            ].map(([rotulo, valor]) => (
              <div key={rotulo} className="card" style={{ padding: 14 }}>
                <div className="t13 mute">{rotulo}</div>
                <div className="num" style={{ fontSize: 20, fontWeight: 650, whiteSpace: 'nowrap' }}>
                  {valor}
                </div>
              </div>
            ))}
          </section>
          <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.6fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
            <section className="card pane" aria-label="Títulos">
              <div className="scroll">
                <Abas
                  rotulo="Títulos"
                  selecionada={aba}
                  aoSelecionar={setAba}
                  abas={[
                    { id: 'pagar', rotulo: `A pagar${r.aguardando_aprovacao > 0 ? ` (${String(r.aguardando_aprovacao)} aguardando aprovação)` : ''}`, conteudo: <ListaDeTitulos tipo="pagar" /> },
                    { id: 'receber', rotulo: 'A receber', conteudo: <ListaDeTitulos tipo="receber" /> },
                    { id: 'repasse', rotulo: 'Repasse', conteudo: <Repasse /> },
                  ]}
                />
              </div>
            </section>
            <section className="card pane" aria-label="Novo lançamento">
              <div className="scroll">
                <NovoLancamento />
              </div>
            </section>
          </div>
        </>
      )}
    </>
  );
}

function ListaDeTitulos({ tipo }: { tipo: 'pagar' | 'receber' }) {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['financeiro', 'titulos', tipo], queryFn: () => api.financeiro.titulos(tipo) });
  const [aberto, setAberto] = useState<Titulo | null>(null);
  return (
    <div className="col" style={{ gap: 10 }}>
      <EstadoDaConsulta oQue="os títulos" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} />
      {q.data !== undefined && (
        <Tabela<Titulo>
          rotulo={tipo === 'pagar' ? 'Títulos a pagar' : 'Títulos a receber'}
          vazio="Nada em aberto."
          linhas={q.data}
          aoAbrir={setAberto}
          colunas={[
            { id: 'contraparte', rotulo: tipo === 'pagar' ? 'Para' : 'De', cabecalhoDaLinha: true, celula: (t) => `${t.contraparte}, ${t.descricao}` },
            { id: 'vencimento', rotulo: 'Vencimento', semQuebra: true, celula: (t) => (t.vencido ? <b style={{ color: 'var(--rose)' }}>{formatarData(t.vencimento)}, vencido</b> : formatarData(t.vencimento)) },
            { id: 'valor', rotulo: 'Valor', direita: true, semQuebra: true, celula: (t) => formatarDinheiro(t.valor_centavos) },
            {
              id: 'situacao',
              rotulo: 'Situação',
              semQuebra: true,
              celula: (t) =>
                t.aprovacoes < t.aprovacoes_exigidas ? (
                  <Chip cor="ambar" texto={`Aprovação ${String(t.aprovacoes)} de ${String(t.aprovacoes_exigidas)}`} />
                ) : (
                  <Chip cor="verde" texto="Pode pagar" />
                ),
            },
          ]}
        />
      )}
      {aberto !== null && <AcoesDoTitulo key={aberto.id} titulo={aberto} />}
    </div>
  );
}

function AcoesDoTitulo({ titulo }: { titulo: Titulo }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const reautenticar = useReautenticar();
  const contas = useQuery({ queryKey: ['financeiro', 'contas'], queryFn: () => api.financeiro.contas() });
  const [conta, setConta] = useState('');
  const atualizar = () => void cliente.invalidateQueries({ queryKey: ['financeiro'] });
  const aprovar = useOperacao((_: undefined, chave: string) => api.financeiro.aprovar(titulo.id, { idempotencyKey: chave }), { aoConcluir: atualizar });
  const baixar = useOperacao(
    (_: undefined, chave: string) => api.financeiro.baixar(titulo.id, { conta_id: conta, data: new Date().toLocaleDateString('sv-SE') }, { idempotencyKey: chave }),
    { aoConcluir: atualizar },
  );
  const precisaAprovar = titulo.aprovacoes < titulo.aprovacoes_exigidas;
  return (
    <div className="col" style={{ gap: 8 }} aria-label={`Título ${titulo.codigo}`}>
      <h3 className="t15 b" style={{ margin: 0 }}>
        {titulo.codigo}: {titulo.contraparte}, {formatarDinheiro(titulo.valor_centavos)}
      </h3>
      {precisaAprovar ? (
        <div className="row">
          <Botao variante="principal" ocupado={aprovar.enviando} onPress={() => { aprovar.enviar(undefined); }}>
            Aprovar
          </Botao>
        </div>
      ) : (
        <div className="row" style={{ gap: 8, alignItems: 'flex-end' }}>
          <div style={{ width: 260 }}>
            <Escolha rotulo="Conta" valor={conta} aoMudar={setConta} vazio="Escolha a conta" opcoes={(contas.data ?? []).map((c) => ({ valor: c.id, rotulo: c.nome }))} />
          </div>
          <Botao variante="principal" ocupado={baixar.enviando} isDisabled={conta === ''} onPress={() => { baixar.enviar(undefined); }}>
            {titulo.tipo === 'pagar' ? 'Registrar pagamento' : 'Registrar recebimento'}
          </Botao>
        </div>
      )}
      {aprovar.erro !== null && (
        <ErroDoAto
          erro={aprovar.erro}
          ato="aprovar o título"
          aoReautenticar={() => {
            reautenticar('aprovar-titulo', aprovar.chave(), {}, () => { aprovar.enviar(undefined); });
          }}
        />
      )}
      {baixar.erro !== null && <ErroDoAto erro={baixar.erro} ato="registrar a baixa" />}
      {aprovar.resultado !== null && <span className="t13" role="status">Aprovação registrada.</span>}
      {baixar.resultado !== null && <span className="t13" role="status">Baixa registrada.</span>}
    </div>
  );
}
