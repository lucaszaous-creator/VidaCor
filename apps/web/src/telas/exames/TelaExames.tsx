import { useQuery } from '@tanstack/react-query';
import { Botao, Busca, Chip, formatarDinheiro, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { CHAVE_CATALOGO } from './comum';
import { DetalheDoExame } from './DetalheDoExame';
import { IncluirExame, NovaTabelaDePrecos } from './FormulariosDoCatalogo';

type Painel = { tipo: 'exame'; tuss: string } | { tipo: 'incluir' } | { tipo: 'tabela' } | null;

function normalizar(t: string): string {
  return t
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLocaleLowerCase('pt-BR');
}

/**
 * Exames e procedimentos (Exames-Procedimentos.dc.html). A recepção consulta o catálogo com o preço
 * vigente; a gerência inclui exame, muda duração, modalidade e ativo com motivo e publica a versão nova
 * da tabela de preços, que começa numa data futura e não altera a vigente. Quem decide é o servidor.
 */
export function TelaExames() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const catalogo = useQuery({
    queryKey: CHAVE_CATALOGO,
    queryFn: () => api.catalogo.listar(),
  });
  const [busca, setBusca] = useState('');
  const [painel, setPainel] = useState<Painel>(null);
  const gerencia = sessao.data?.usuario.perfis.includes('gerente') ?? false;
  const c = catalogo.data;
  const vigente = c?.tabelas.find((t) => t.situacao === 'vigente');
  const futura = c?.tabelas.find((t) => t.situacao === 'futura');
  const ativos = c?.procedimentos.filter((p) => p.ativo).length ?? 0;
  const exames = (c?.procedimentos ?? []).filter((p) => busca.trim() === '' || normalizar(`${p.nome} ${p.tuss}`).includes(normalizar(busca.trim())));
  const atual = painel?.tipo === 'exame' ? (c?.procedimentos.find((p) => p.tuss === painel.tuss) ?? null) : null;
  return (
    <>
      <CabecalhoDaTela
        titulo="Exames e procedimentos"
        complemento={
          c === undefined ? undefined : (
            <span className="t14 mute">
              {ativos} exames ativos, {vigente === undefined ? 'sem tabela vigente' : `tabela v${String(vigente.versao)} vigente`}
            </span>
          )
        }
      >
        {gerencia && (
          <>
            <Botao
              pequeno
              onPress={() => {
                setPainel({ tipo: 'tabela' });
              }}
            >
              Nova tabela de preços
            </Botao>
            <Botao
              pequeno
              variante="principal"
              onPress={() => {
                setPainel({ tipo: 'incluir' });
              }}
            >
              Incluir exame
            </Botao>
          </>
        )}
      </CabecalhoDaTela>
      <div
        className="fill"
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)',
          gridTemplateRows: 'minmax(0, 1fr)',
          gap: 16,
        }}
      >
        <section className="card pane" aria-label="Catálogo de exames" style={{ padding: 0 }}>
          <div style={{ padding: '14px 16px 8px' }}>
            <Busca rotulo="Buscar exame" valor={busca} aoMudar={setBusca} placeholder="Nome ou código TUSS" />
          </div>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="o catálogo de exames"
              carregando={catalogo.isPending}
              erro={catalogo.error}
              aoTentarDeNovo={() => void catalogo.refetch()}
              semPermissao={{
                titulo: 'Você não consulta o catálogo',
                detalhe: 'O catálogo de exames é da recepção e da gerência.',
              }}
            />
            {c !== undefined && exames.length === 0 && <Vazio titulo="Nenhum exame com esse nome." detalhe="Busque pelo código TUSS ou por parte do nome." />}
            <ul style={{ listStyle: 'none', margin: 0, padding: '0 8px 8px' }}>
              {exames.map((p) => (
                <li key={p.tuss}>
                  <button
                    type="button"
                    className="lp"
                    aria-current={painel?.tipo === 'exame' && painel.tuss === p.tuss ? 'true' : undefined}
                    onClick={() => {
                      setPainel({ tipo: 'exame', tuss: p.tuss });
                    }}
                  >
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <span className="t clamp1" style={{ display: 'block' }}>
                        {p.nome}
                      </span>
                      <span className="s clamp1" style={{ display: 'block' }}>
                        TUSS <span className="num">{p.tuss}</span>, {p.modalidade ?? 'sem modalidade'}, {p.duracao_min} min
                      </span>
                    </span>
                    <span className="num t13">{typeof p.preco_centavos === 'number' ? formatarDinheiro(p.preco_centavos) : 'Sem preço'}</span>
                    {!p.ativo && <Chip cor="cinza" texto="Inativo" />}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do exame">
          <div className="scroll col" style={{ gap: 14 }}>
            {painel?.tipo === 'incluir' && (
              <IncluirExame
                aoIncluir={(tuss) => {
                  setPainel({ tipo: 'exame', tuss });
                }}
                aoFechar={() => {
                  setPainel(null);
                }}
              />
            )}
            {painel?.tipo === 'tabela' && c !== undefined && (
              <NovaTabelaDePrecos
                catalogo={c}
                aoFechar={() => {
                  setPainel(null);
                }}
              />
            )}
            {painel?.tipo === 'exame' && atual !== null && <DetalheDoExame key={atual.tuss} p={atual} gerencia={gerencia} proximaVigencia={futura?.vigente_desde ?? null} />}
            {(painel === null || (painel.tipo === 'exame' && atual === null)) && (
              <Vazio titulo="Escolha um exame." detalhe="Duração, modalidade, preço vigente e o histórico de mudanças aparecem aqui." />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
