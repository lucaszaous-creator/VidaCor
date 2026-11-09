import { useQuery } from '@tanstack/react-query';
import type { ModeloDeLaudo } from '@vidacor/api-client';
import { Abas, Botao, Busca, Chip, Escolha, formatarData, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { CHAVE_MODELOS, EditorDeModelo, NovoModelo, QuemPodeEditar, tipoDoModelo, VersoesDoModelo } from './EditorDeModelo';
import { FrasesRapidas } from './FrasesRapidas';

const sem = (t: string): string =>
  t
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase();

/** O que é longo rola dentro do painel; a página não rola (docs/05). */
const ALTURA_DA_ABA = { height: 'calc(100vh - 196px)', minHeight: 300, paddingTop: 10 } as const;
const ALTURA_DO_DETALHE = { height: 'calc(100vh - 360px)', minHeight: 140, paddingTop: 8 } as const;

function Detalhe({ m }: { m: ModeloDeLaudo }) {
  const t = tipoDoModelo(m);
  const publicada = m.versoes.find((v) => v.situacao === 'publicada');
  return (
    <div className="col" style={{ gap: 10, minHeight: 0, height: '100%' }}>
      <div className="row wrap" style={{ gap: 10 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {m.nome}
        </h2>
        <Chip cor={t.cor} texto={t.texto} />
        {!m.ativo && <Chip cor="cinza" texto="Desativado" />}
      </div>
      <p className="t13 mute" style={{ margin: 0 }}>
        {m.exame}, {m.modalidade}.{' '}
        {publicada === undefined
          ? 'Ainda sem versão publicada.'
          : `Versão ${String(publicada.numero)} de ${formatarData((publicada.decidido_em ?? publicada.criado_em).slice(0, 10))}, usada em ${String(m.usos)} laudos.`}{' '}
        <span className="num">{m.codigo}</span>
      </p>
      <div>
        <Abas
          rotulo={`Modelo ${m.nome}`}
          abas={[
            { id: 'editor', rotulo: 'Editor', conteudo: <div className="scroll" style={ALTURA_DO_DETALHE}><EditorDeModelo m={m} /></div> },
            { id: 'versoes', rotulo: `Versões (${String(m.versoes.length)})`, conteudo: <div className="scroll" style={ALTURA_DO_DETALHE}><VersoesDoModelo m={m} /></div> },
            { id: 'quem', rotulo: 'Quem pode editar', conteudo: <div className="scroll" style={ALTURA_DO_DETALHE}><QuemPodeEditar /></div> },
          ]}
        />
      </div>
    </div>
  );
}

function Modelos({ modelos }: { modelos: ModeloDeLaudo[] }) {
  const [filtro, setFiltro] = useState('');
  const [busca, setBusca] = useState('');
  const [escolhido, setEscolhido] = useState<string | null>(null);
  const [criando, setCriando] = useState(false);
  const visiveis = modelos.filter((m) => (filtro === '' || m.tipo === filtro) && (busca.trim() === '' || sem(`${m.nome} ${m.exame}`).includes(sem(busca.trim()))));
  const grupos = [...new Set(visiveis.map((m) => m.modalidade))];
  const atual = modelos.find((m) => m.id === escolhido) ?? null;
  return (
    <div style={{ display: 'grid', gridTemplateColumns: '320px minmax(0, 1fr)', gap: 16, height: '100%', minHeight: 0 }}>
      <section className="col" aria-label="Lista de modelos" style={{ gap: 8, minHeight: 0 }}>
        <div className="row" style={{ gap: 8, alignItems: 'flex-end' }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <Escolha
              rotulo="Mostrar"
              valor={filtro}
              aoMudar={setFiltro}
              vazio="Todos"
              opcoes={[
                { valor: 'clinica', rotulo: 'Da clínica' },
                { valor: 'pessoal', rotulo: 'Pessoais' },
              ]}
            />
          </div>
          <Botao
            variante="principal"
            onPress={() => {
              setCriando(true);
              setEscolhido(null);
            }}
          >
            Novo modelo
          </Botao>
        </div>
        <Busca rotulo="Buscar modelo" valor={busca} aoMudar={setBusca} placeholder="Nome ou exame" />
        <div className="scroll" style={{ minHeight: 0 }}>
          {visiveis.length === 0 && <Vazio titulo="Nenhum modelo encontrado" detalhe="Confira o nome do exame." />}
          {grupos.map((g) => (
            <div key={g}>
              <h3 className="t13 mute" style={{ margin: '8px 0 4px' }}>
                {g}
              </h3>
              <ul style={{ listStyle: 'none', margin: 0, padding: 0 }} aria-label={`Modelos de ${g}`}>
                {visiveis
                  .filter((m) => m.modalidade === g)
                  .map((m) => (
                    <li key={m.id}>
                      <button
                        type="button"
                        className={`linha-paciente${m.id === escolhido ? ' sel' : ''}`}
                        aria-current={m.id === escolhido ? 'true' : undefined}
                        style={{ width: '100%', textAlign: 'left', display: 'flex', gap: 8, alignItems: 'center', padding: '8px', border: 0, background: 'transparent', borderBottom: '1px solid var(--line)', cursor: 'pointer', minHeight: 44 }}
                        onClick={() => {
                          setEscolhido(m.id);
                          setCriando(false);
                        }}
                      >
                        <span style={{ flex: 1, minWidth: 0 }}>
                          <b style={{ display: 'block', overflowWrap: 'anywhere' }}>{m.nome}</b>
                          <span className="t13 mute">
                            {m.autor}
                            {m.versao_publicada === null || m.versao_publicada === undefined ? '' : `, v${String(m.versao_publicada)}`}
                          </span>
                        </span>
                        {m.proposta_em_aberto !== null && m.proposta_em_aberto !== undefined && <Chip cor="ambar" texto="Proposta" />}
                        {!m.ativo && <Chip cor="cinza" texto="Desativado" />}
                      </button>
                    </li>
                  ))}
              </ul>
            </div>
          ))}
        </div>
      </section>
      <section className="card pane" aria-label="Detalhe do modelo" style={{ minHeight: 0 }}>
        {criando ? (
          <div className="scroll">
            <NovoModelo
              aoFechar={() => {
                setCriando(false);
              }}
              aoCriar={(id) => {
                setCriando(false);
                setEscolhido(id);
              }}
            />
          </div>
        ) : atual === null ? (
          <Vazio titulo="Escolha um modelo." detalhe="O texto, as versões e quem pode editar aparecem aqui." />
        ) : (
          <Detalhe key={atual.id} m={atual} />
        )}
      </section>
    </div>
  );
}

/**
 * Modelos de laudo e frases rápidas (Modelos-Laudo.dc.html). Para o médico: os modelos da clínica e
 * os pessoais; o servidor diz o que cada um pode editar, propor ou aprovar (coordenação médica).
 */
export function TelaModelosDeLaudo() {
  const { api } = useDependencias();
  const modelos = useQuery({ queryKey: CHAVE_MODELOS, queryFn: () => api.modelosDeLaudo.listar() });
  const sessao = useQuery(consultaSessao(api));
  const perfis = sessao.data?.usuario.perfis ?? [];
  const coordenacao = perfis.includes('medico') && perfis.includes('gerente');
  const propostas = (modelos.data ?? []).filter((m) => m.pode_decidir).length;
  return (
    <>
      <CabecalhoDaTela
        titulo="Modelos de laudo"
        complemento={
          <span className="t14 mute">
            {coordenacao
              ? `Coordenação médica: ${String(propostas)} proposta(s) aguardando sua decisão.`
              : 'Você edita os seus modelos e propõe mudança nos da clínica.'}
          </span>
        }
      />
      <section className="card pane fill" aria-label="Modelos e frases">
        {modelos.data === undefined ? (
          <EstadoDaConsulta
            oQue="os modelos de laudo"
            carregando={modelos.isPending}
            erro={modelos.error}
            aoTentarDeNovo={() => void modelos.refetch()}
            semPermissao={{ titulo: 'Seu perfil não usa modelos de laudo', detalhe: 'Modelos e frases são da equipe médica.' }}
          />
        ) : (
          <Abas
            rotulo="Modelos e frases"
            abas={[
              { id: 'modelos', rotulo: 'Modelos', conteudo: <div style={ALTURA_DA_ABA}><Modelos modelos={modelos.data} /></div> },
              { id: 'frases', rotulo: 'Frases rápidas', conteudo: <div style={ALTURA_DA_ABA}><FrasesRapidas coordenacao={coordenacao} /></div> },
            ]}
          />
        )}
      </section>
    </>
  );
}
