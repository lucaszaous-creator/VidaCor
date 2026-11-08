import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { MotivoDeDesconto, NovoOrcamento, Orcamento, PacienteResumo, PagamentoPrevisto } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Chip, Escolha, formatarData, formatarDinheiro, formatarPontosBase, ListaDeDados, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { EscolhaDePaciente } from '../../componentes/EscolhaDePaciente';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const MOTIVOS: { valor: MotivoDeDesconto; rotulo: string }[] = [
  { valor: 'paciente_frequente', rotulo: 'Paciente frequente' },
  { valor: 'funcionario_ou_familiar', rotulo: 'Funcionário ou familiar' },
  { valor: 'condicao_social', rotulo: 'Condição social avaliada' },
  { valor: 'cobrir_outra_clinica', rotulo: 'Cobrir orçamento de outra clínica' },
];
const PAGAMENTOS: { valor: PagamentoPrevisto; rotulo: string }[] = [
  { valor: 'pix', rotulo: 'PIX' },
  { valor: 'cartao', rotulo: 'Cartão' },
  { valor: 'dinheiro', rotulo: 'Dinheiro' },
];
const SITUACAO: Record<Orcamento['situacao'], { texto: string; cor: 'verde' | 'cinza' | 'ambar' }> = {
  emitido: { texto: 'Emitido', cor: 'verde' },
  vencido: { texto: 'Vencido', cor: 'ambar' },
  cancelado: { texto: 'Cancelado', cor: 'cinza' },
};

/** "5" ou "5,5" → pontos base; vazio → sem desconto; inválido → null. */
function lerPercentual(texto: string): number | null | undefined {
  const t = texto.trim().replace(',', '.');
  if (t === '') return undefined;
  if (!/^\d{1,3}(\.\d{1,2})?$/.test(t)) return null;
  return Math.round(Number(t) * 100);
}

function Resultado({ o, aoNovo }: { o: Orcamento; aoNovo: () => void }) {
  return (
    <Aviso
      nivel="feito"
      titulo={`Orçamento ${o.codigo} emitido para ${o.paciente_nome}.`}
      acoes={
        <Botao pequeno onPress={aoNovo}>
          Novo orçamento
        </Botao>
      }
    >
      Total {formatarDinheiro(o.total_centavos)} pela tabela v{o.tabela_versao}
      {o.pacote_regra_versao === null || o.pacote_regra_versao === undefined ? '' : ` e regra de pacote v${String(o.pacote_regra_versao)}`}. Válido até{' '}
      {formatarData(o.valido_ate)}; depois vale a tabela do dia.
    </Aviso>
  );
}

/**
 * Orçamento do particular (Orcamento.dc.html). A tela monta o pedido; preço, pacote, desconto e total
 * são do servidor (G1). A prévia usa os preços que o servidor mandou e é refeita por ele ao emitir.
 */
export function TelaOrcamento() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const catalogo = useQuery({ queryKey: ['orcamento-catalogo'], queryFn: () => api.orcamentos.catalogo() });
  const recentes = useQuery({ queryKey: ['orcamentos'], queryFn: () => api.orcamentos.listar() });
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [busca, setBusca] = useState('');
  const [itens, setItens] = useState<string[]>([]);
  const [usarPacote, setUsarPacote] = useState(false);
  const [percentual, setPercentual] = useState('');
  const [motivo, setMotivo] = useState('');
  const [pagamento, setPagamento] = useState<string>('pix');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoOrcamento, chave: string) => api.orcamentos.emitir(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['orcamentos'] }),
  });

  const c = catalogo.data;
  const escolhidos = (c?.exames ?? []).filter((e) => itens.includes(e.tuss));
  const achados = busca.trim().length < 2 ? [] : (c?.exames ?? []).filter((e) => !itens.includes(e.tuss) && e.nome.toLowerCase().includes(busca.trim().toLowerCase())).slice(0, 6);
  const pacote = (c?.pacotes ?? []).find((p) => p.tuss.every((t) => itens.includes(t)));
  const subtotal = escolhidos.reduce((s, e) => s + e.valor_centavos, 0);
  const somaDoPacote = pacote === undefined ? 0 : escolhidos.filter((e) => pacote.tuss.includes(e.tuss)).reduce((s, e) => s + e.valor_centavos, 0);
  const economia = pacote !== undefined && usarPacote ? Math.max(0, somaDoPacote - pacote.valor_centavos) : 0;
  const pb = lerPercentual(percentual);
  const limite = c?.limite_desconto_pontos_base ?? 1000;
  const acimaDoLimite = typeof pb === 'number' && pb > limite;
  const descontoValido = typeof pb === 'number' && pb > 0 && !acimaDoLimite && motivo !== '';
  const desconto = descontoValido ? Math.floor(((subtotal - economia) * pb) / 10_000) : 0;
  const erroDoPercentual = pb === null ? 'Use número, ex.: 5 ou 7,5.' : acimaDoLimite ? `Passa do limite de ${formatarPontosBase(limite)} do orçamento. Peça o desconto maior no caixa, com autorização da gerência.` : null;

  function novo(): void {
    op.reiniciar();
    setItens([]);
    setUsarPacote(false);
    setPercentual('');
    setMotivo('');
    setTentou(false);
  }

  function emitir(): void {
    setTentou(true);
    if (paciente === null || itens.length === 0 || pb === null || acimaDoLimite || (typeof pb === 'number' && motivo === '')) return;
    op.enviar({
      paciente_id: paciente.id,
      itens,
      pacote: pacote !== undefined && usarPacote ? pacote.codigo : null,
      desconto: typeof pb === 'number' && pb > 0 ? { pontos_base: pb, motivo: motivo as MotivoDeDesconto } : null,
      pagamento_previsto: pagamento as PagamentoPrevisto,
    });
  }

  return (
    <>
      <CabecalhoDaTela titulo="Orçamento particular" complemento={<span className="t14 mute">Preços da tabela vigente{c === undefined ? '' : ` (v${String(c.tabela_versao)})`}</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.25fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Novo orçamento">
          <div className="scroll col" style={{ gap: 12 }}>
            <EstadoDaConsulta oQue="a tabela de preços" carregando={catalogo.isPending} erro={catalogo.error} aoTentarDeNovo={() => void catalogo.refetch()} />
            <EscolhaDePaciente rotulo="Para quem" paciente={paciente} aoEscolher={setPaciente} erro={tentou && paciente === null ? 'Escolha o paciente do cadastro.' : null} />
            <Campo rotulo="Buscar exame para adicionar" type="search" value={busca} onChange={setBusca} ajuda="Pelo nome do exame que está no pedido médico." />
            {achados.length > 0 && (
              <ul className="col" style={{ gap: 4, listStyle: 'none', margin: 0, padding: 0 }} aria-label="Exames encontrados">
                {achados.map((e) => (
                  <li key={e.tuss}>
                    <Botao
                      pequeno
                      aria-label={`Adicionar ${e.nome}, ${formatarDinheiro(e.valor_centavos)}`}
                      onPress={() => {
                        setItens([...itens, e.tuss]);
                        setBusca('');
                      }}
                    >
                      {e.nome}, {formatarDinheiro(e.valor_centavos)}
                    </Botao>
                  </li>
                ))}
              </ul>
            )}
            <h2 className="t15 b" style={{ margin: 0 }}>
              Exames
            </h2>
            {escolhidos.length === 0 ? (
              <Vazio titulo="Nenhum exame no orçamento" detalhe="Busque pelo nome do exame que está no pedido médico." />
            ) : (
              <ul className="col" style={{ gap: 6, listStyle: 'none', margin: 0, padding: 0 }} aria-label="Exames do orçamento">
                {escolhidos.map((e) => (
                  <li key={e.tuss} className="row" style={{ gap: 8 }}>
                    <span style={{ flex: 1, minWidth: 0 }}>{e.nome}</span>
                    <span className="num">{formatarDinheiro(e.valor_centavos)}</span>
                    <Botao
                      pequeno
                      aria-label={`Remover ${e.nome}`}
                      onPress={() => {
                        setItens(itens.filter((t) => t !== e.tuss));
                      }}
                    >
                      Remover
                    </Botao>
                  </li>
                ))}
              </ul>
            )}
            {tentou && itens.length === 0 && <span className="campo-erro">Inclua ao menos um exame.</span>}
            {pacote !== undefined &&
              (usarPacote ? (
                <Aviso nivel="informativo" titulo={`${pacote.nome} aplicado (regra de combinação v${String(pacote.regra_versao)}).`} acoes={<Botao pequeno onPress={() => { setUsarPacote(false); }}>Desfazer</Botao>}>
                  Prévia: menos {formatarDinheiro(economia)}.
                </Aviso>
              ) : (
                <Aviso nivel="informativo" titulo={`${pacote.nome}: estes exames saem por ${formatarDinheiro(pacote.valor_centavos)} em vez de ${formatarDinheiro(somaDoPacote)}.`} acoes={<Botao pequeno onPress={() => { setUsarPacote(true); }}>Aplicar pacote</Botao>}>
                  O servidor confere a regra ao emitir.
                </Aviso>
              ))}
            <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 0.7fr) minmax(0, 1.3fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
              <Campo rotulo="Desconto (%)" value={percentual} onChange={setPercentual} ajuda={`Recepção concede até ${formatarPontosBase(limite)} com motivo.`} erro={erroDoPercentual} />
              <Escolha rotulo="Motivo do desconto" valor={motivo} aoMudar={setMotivo} vazio="Escolha o motivo" opcoes={MOTIVOS} erro={typeof pb === 'number' && pb > 0 && motivo === '' ? 'Sem motivo o desconto não vale.' : null} />
              <Escolha rotulo="Pagamento previsto" valor={pagamento} aoMudar={setPagamento} opcoes={PAGAMENTOS} />
            </div>
          </div>
        </section>
        <div className="col" style={{ gap: 16, minHeight: 0 }}>
          <section className="card pane" aria-label="Resumo">
            <h2 className="t15 b" style={{ margin: '0 0 8px' }}>
              Resumo
            </h2>
            <ListaDeDados
              larguraDoRotulo={140}
              itens={[
                { rotulo: `Exames (${String(escolhidos.length)})`, valor: <span className="num">{formatarDinheiro(subtotal)}</span> },
                ...(economia > 0 ? [{ rotulo: 'Pacote', valor: <span className="num">− {formatarDinheiro(economia)}</span> }] : []),
                ...(desconto > 0 && typeof pb === 'number' ? [{ rotulo: `Desconto ${formatarPontosBase(pb)}`, valor: <span className="num">− {formatarDinheiro(desconto)}</span> }] : []),
                { rotulo: 'Total (prévia)', valor: <b className="num">{formatarDinheiro(subtotal - economia - desconto)}</b> },
              ]}
            />
            <p className="t13 mute" style={{ margin: '8px 0' }}>
              O valor que vale é o do orçamento emitido, calculado pelo servidor. Válido por 14 dias.
            </p>
            <div aria-live="polite" className="col" style={{ gap: 8 }}>
              {op.resultado !== null && <Resultado o={op.resultado} aoNovo={novo} />}
              {op.erro !== null && <ErroDoAto erro={op.erro} ato="emitir o orçamento" />}
            </div>
            {op.resultado === null && (
              <div className="row" style={{ marginTop: 8 }}>
                <Botao variante="principal" ocupado={op.enviando} onPress={emitir}>
                  Emitir orçamento
                </Botao>
              </div>
            )}
          </section>
          <section className="card pane" aria-label="Orçamentos recentes" style={{ flex: 1, minHeight: 0 }}>
            <h2 className="t15 b" style={{ margin: '0 0 8px' }}>
              Últimos 30 dias
            </h2>
            <div className="scroll">
              <EstadoDaConsulta oQue="os orçamentos" carregando={recentes.isPending} erro={recentes.error} aoTentarDeNovo={() => void recentes.refetch()} />
              {recentes.data?.length === 0 && <Vazio titulo="Nenhum orçamento nos últimos 30 dias." />}
              <ul className="col" style={{ gap: 6, listStyle: 'none', margin: 0, padding: 0 }}>
                {recentes.data?.map((o) => (
                  <li key={o.id} className="row" style={{ gap: 10, alignItems: 'flex-start', padding: '6px 0', borderBottom: '1px solid var(--line)' }}>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div className="b" style={{ overflowWrap: 'anywhere' }}>{o.paciente_nome}</div>
                      <div className="t13 mute">
                        <span className="num">{o.codigo}</span>, nascimento {formatarData(o.paciente_nascimento)}
                      </div>
                    </div>
                    <div className="col" style={{ gap: 4, alignItems: 'flex-end' }}>
                      <span className="num b">{formatarDinheiro(o.total_centavos)}</span>
                      <Chip cor={SITUACAO[o.situacao].cor} texto={SITUACAO[o.situacao].texto} />
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          </section>
        </div>
      </div>
    </>
  );
}
