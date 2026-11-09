import { useQuery } from '@tanstack/react-query';
import type { ItemDaFilaDeRevisao } from '@vidacor/api-client';
import { Abas, Chip, formatarData, Vazio, type CorDoChip } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { CasoDeRevisao, CHAVE_FILA_REVISAO } from './CasoDeRevisao';
import { MeusLaudosRevisados, ResultadosDaRevisao } from './ResultadosDaRevisao';

const SITUACAO: Record<ItemDaFilaDeRevisao['situacao'], { texto: string; cor: CorDoChip }> = {
  designada: { texto: 'A revisar', cor: 'ambar' },
  concluida: { texto: 'Feita', cor: 'verde' },
  conflito_declarado: { texto: 'Conflito declarado', cor: 'cinza' },
};

/** O que é longo rola dentro do painel; a página não rola (docs/05). */
const ALTURA = { height: 'calc(100vh - 196px)', minHeight: 300, paddingTop: 10 } as const;

function FilaDoRevisor() {
  const { api } = useDependencias();
  const fila = useQuery({ queryKey: CHAVE_FILA_REVISAO, queryFn: () => api.revisaoPorPares.fila() });
  const [escolhida, setEscolhida] = useState<string | null>(null);
  const itens = fila.data ?? [];
  const feitas = itens.filter((i) => i.situacao !== 'designada').length;
  return (
    <div style={{ ...ALTURA, display: 'grid', gridTemplateColumns: '300px minmax(0, 1fr)', gap: 16 }}>
      <section className="col" aria-label="Minha fila de revisão" style={{ gap: 6, minHeight: 0 }}>
        <h3 className="t14 b" style={{ margin: 0 }}>
          Minha fila, {feitas} de {itens.length} feitas
        </h3>
        <div className="scroll" style={{ minHeight: 0 }}>
          <EstadoDaConsulta
            oQue="a fila de revisão"
            carregando={fila.isPending}
            erro={fila.error}
            aoTentarDeNovo={() => void fila.refetch()}
            semPermissao={{ titulo: 'Seu perfil não revisa laudos', detalhe: 'A revisão por pares é da equipe médica.' }}
          />
          {fila.data?.length === 0 && <Vazio titulo="Nenhum laudo para você revisar." detalhe="A coordenação médica sorteia os casos." />}
          <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
            {itens.map((i) => (
              <li key={i.id}>
                <button
                  type="button"
                  className={`linha-paciente${i.id === escolhida ? ' sel' : ''}`}
                  aria-current={i.id === escolhida ? 'true' : undefined}
                  style={{ width: '100%', textAlign: 'left', display: 'flex', gap: 8, alignItems: 'center', padding: 8, border: 0, background: 'transparent', borderBottom: '1px solid var(--line)', cursor: 'pointer', minHeight: 44 }}
                  onClick={() => {
                    setEscolhida(i.id);
                  }}
                >
                  <span style={{ flex: 1, minWidth: 0 }}>
                    <b style={{ display: 'block', overflowWrap: 'anywhere' }}>{i.exame}</b>
                    <span className="t13 mute num">
                      {i.codigo}, {formatarData(i.designada_em.slice(0, 10))}
                    </span>
                  </span>
                  <Chip cor={SITUACAO[i.situacao].cor} texto={SITUACAO[i.situacao].texto} />
                </button>
              </li>
            ))}
          </ul>
        </div>
      </section>
      <section className="card pane" aria-label="Caso em revisão" style={{ minHeight: 0 }}>
        <div className="scroll">
          {escolhida === null ? <Vazio titulo="Escolha um caso da fila." detalhe="O autor fica oculto até o envio. A abertura do laudo fica registrada." /> : <CasoDeRevisao key={escolhida} id={escolhida} />}
        </div>
      </section>
    </div>
  );
}

/**
 * Revisão por pares (Revisao-Pares.dc.html): fila do revisor com parecer RADPEER, revisões dos laudos
 * do próprio médico e, para a coordenação médica, sorteio e resultados. O servidor decide quem revisa
 * (nunca o autor) e o que cada um vê.
 */
export function TelaRevisaoPorPares() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const perfis = sessao.data?.usuario.perfis ?? [];
  const coordenacao = perfis.includes('medico') && perfis.includes('gerente');
  const [aba, setAba] = useState('fila');
  return (
    <>
      <CabecalhoDaTela
        titulo="Revisão por pares"
        complemento={
          <span className="t14 mute">
            {aba === 'resultados'
              ? 'Confidencial: só o próprio médico e a coordenação médica veem o resultado individual.'
              : 'Laudos de colegas sorteados para você. O autor fica oculto até o envio.'}
          </span>
        }
      />
      <section className="card pane fill" aria-label="Revisão por pares">
        <Abas
          rotulo="Visões da revisão"
          selecionada={aba}
          aoSelecionar={setAba}
          abas={[
            { id: 'fila', rotulo: 'Fila do revisor', conteudo: <FilaDoRevisor /> },
            {
              id: 'meus',
              rotulo: 'Revisões dos meus laudos',
              conteudo: (
                <div className="scroll" style={ALTURA}>
                  <MeusLaudosRevisados />
                </div>
              ),
            },
            ...(coordenacao
              ? [
                  {
                    id: 'resultados',
                    rotulo: 'Resultados',
                    conteudo: (
                      <div className="scroll" style={ALTURA}>
                        <ResultadosDaRevisao />
                      </div>
                    ),
                  },
                ]
              : []),
          ]}
        />
      </section>
    </>
  );
}
