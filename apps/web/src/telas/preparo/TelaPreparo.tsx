import { useQuery } from '@tanstack/react-query';
import type { PreparoDoExame } from '@vidacor/api-client';
import { Botao, Busca, Chip, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { CHAVE_PREPAROS, NovaVersaoDoPreparo } from './FormularioDoPreparo';

function normalizar(t: string): string {
  return t
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLocaleLowerCase('pt-BR');
}

function Versoes({ tuss }: { tuss: string }) {
  const { api } = useDependencias();
  const versoes = useQuery({
    queryKey: [...CHAVE_PREPAROS, tuss],
    queryFn: () => api.preparos.versoes(tuss),
  });
  return (
    <section aria-label="Versões do preparo" className="col" style={{ gap: 8 }}>
      <h3 className="t14 b" style={{ margin: 0 }}>
        Versões
      </h3>
      <EstadoDaConsulta oQue="as versões do preparo" carregando={versoes.isPending} erro={versoes.error} aoTentarDeNovo={() => void versoes.refetch()} />
      <ol style={{ listStyle: 'none', margin: 0, padding: 0 }}>
        {(versoes.data ?? []).map((v) => (
          <li key={v.id} style={{ padding: '8px 0', borderBottom: '1px solid var(--linha)' }}>
            <div className="row" style={{ gap: 8, alignItems: 'baseline', flexWrap: 'wrap' }}>
              <span className="t14 b">Versão {v.numero}</span>
              {v.vigente && <Chip cor="verde" texto="Vigente" />}
              <span className="t13 mute">
                {formatarDataHora(v.criado_em)}, {v.criado_por}
              </span>
            </div>
            <p className="t13" style={{ margin: '4px 0 0', overflowWrap: 'anywhere' }}>
              {v.motivo ?? 'Primeira versão.'} {v.passos.length} {v.passos.length === 1 ? 'passo' : 'passos'}, chegar {v.chegar_antes_min} min antes.
            </p>
          </li>
        ))}
      </ol>
    </section>
  );
}

function Detalhe({ exame, gerencia }: { exame: PreparoDoExame; gerencia: boolean }) {
  const [editando, setEditando] = useState(false);
  const v = exame.vigente ?? null;
  if (editando) {
    return (
      <NovaVersaoDoPreparo
        exame={exame}
        aoFechar={() => {
          setEditando(false);
        }}
      />
    );
  }
  return (
    <>
      <div className="row" style={{ gap: 8, alignItems: 'flex-start' }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h2 className="t17 b" style={{ margin: 0 }}>
            {exame.nome}
          </h2>
          <p className="t13 mute" style={{ margin: '2px 0 0' }}>
            TUSS <span className="num">{exame.tuss}</span>
            {v !== null && `, versão ${String(v.numero)}, chegar ${String(v.chegar_antes_min)} min antes`}
          </p>
        </div>
        {gerencia && (
          <Botao
            pequeno
            variante="principal"
            onPress={() => {
              setEditando(true);
            }}
          >
            {v === null ? 'Cadastrar preparo' : 'Nova versão'}
          </Botao>
        )}
      </div>
      {v === null ? (
        <Vazio
          titulo="Sem orientação de preparo."
          detalhe="O comprovante sai dizendo que não há orientação cadastrada e pede que o paciente fale com a recepção em caso de dúvida."
        />
      ) : (
        <ol aria-label="Passos do preparo vigente" className="col" style={{ gap: 6, margin: 0, paddingLeft: 22 }}>
          {v.passos.map((p, i) => (
            <li key={`${String(i)}-${p}`} className="t14" style={{ overflowWrap: 'anywhere' }}>
              {p}
            </li>
          ))}
        </ol>
      )}
      <Versoes tuss={exame.tuss} />
    </>
  );
}

/**
 * Preparo de exames (Preparo-Exame.dc.html). A recepção consulta a orientação que vai no comprovante
 * do agendamento; a gerência publica versão nova com motivo. Nada é apagado: a versão anterior fica
 * guardada e o comprovante já entregue continua com o texto que imprimiu.
 */
export function TelaPreparo() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const preparos = useQuery({ queryKey: CHAVE_PREPAROS, queryFn: () => api.preparos.listar() });
  const [busca, setBusca] = useState('');
  const [tuss, setTuss] = useState<string | null>(null);
  const gerencia = sessao.data?.usuario.perfis.includes('gerente') ?? false;
  const lista = preparos.data ?? [];
  const sem = lista.filter((p) => p.vigente == null).length;
  const filtrados = lista.filter((p) => busca.trim() === '' || normalizar(`${p.nome} ${p.tuss}`).includes(normalizar(busca.trim())));
  const atual = lista.find((p) => p.tuss === tuss) ?? null;
  return (
    <>
      <CabecalhoDaTela
        titulo="Preparo de exames"
        complemento={
          preparos.data === undefined ? undefined : (
            <span className="t14 mute">{sem === 0 ? 'Todos os exames têm orientação' : `${String(sem)} ${sem === 1 ? 'exame' : 'exames'} sem orientação`}</span>
          )
        }
      />
      <div
        className="fill"
        style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}
      >
        <section className="card pane" aria-label="Exames" style={{ padding: 0 }}>
          <div style={{ padding: '14px 16px 8px' }}>
            <Busca rotulo="Buscar exame" valor={busca} aoMudar={setBusca} placeholder="Nome ou código TUSS" />
          </div>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="o preparo dos exames"
              carregando={preparos.isPending}
              erro={preparos.error}
              aoTentarDeNovo={() => void preparos.refetch()}
              semPermissao={{ titulo: 'Você não consulta o preparo', detalhe: 'O preparo dos exames é da recepção e da gerência.' }}
            />
            {preparos.data !== undefined && filtrados.length === 0 && <Vazio titulo="Nenhum exame com esse nome." detalhe="Busque pelo código TUSS ou por parte do nome." />}
            <ul style={{ listStyle: 'none', margin: 0, padding: '0 8px 8px' }}>
              {filtrados.map((p) => (
                <li key={p.tuss}>
                  <button
                    type="button"
                    className="lp"
                    aria-current={tuss === p.tuss ? 'true' : undefined}
                    onClick={() => {
                      setTuss(p.tuss);
                    }}
                  >
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <span className="t clamp1" style={{ display: 'block' }}>
                        {p.nome}
                      </span>
                      <span className="s clamp1" style={{ display: 'block' }}>
                        TUSS <span className="num">{p.tuss}</span>
                        {p.vigente != null && `, ${String(p.vigente.passos.length)} passos, versão ${String(p.vigente.numero)}`}
                      </span>
                    </span>
                    {p.vigente == null && <Chip cor="ambar" texto="Sem orientação" />}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do preparo">
          <div className="scroll col" style={{ gap: 14 }}>
            {atual === null ? (
              <Vazio titulo="Escolha um exame." detalhe="Os passos que o paciente recebe no comprovante e as versões anteriores aparecem aqui." />
            ) : (
              <Detalhe key={atual.tuss} exame={atual} gerencia={gerencia} />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
