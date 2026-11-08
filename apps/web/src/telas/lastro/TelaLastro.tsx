import { useQuery } from '@tanstack/react-query';
import { Aviso, Botao, Busca, Chip, formatarData, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const NOMES: Record<string, string> = {
  agendamento: 'Agendamento',
  guia: 'Guia do convênio',
  atendimento: 'Atendimento',
  etiqueta: 'Etiqueta',
  triagem: 'Triagem',
  estudo: 'Estudo de imagem',
  laudo: 'Laudo',
  recebimento: 'Recebimento',
  estorno: 'Estorno',
  contraste: 'Lote de contraste',
};

/**
 * Explorador de lastro (Lastro.dc.html). Qualquer código do atendimento abre a cadeia: registros
 * ligados e eventos imutáveis, com a conferência do hash feita pelo servidor. Mostra o caminho, não
 * o conteúdo; abrir a cadeia fica registrado no lastro.
 */
export function TelaLastro() {
  const { api } = useDependencias();
  const [texto, setTexto] = useState('');
  const [codigo, setCodigo] = useState<string | null>(null);
  const cadeia = useQuery({ queryKey: ['lastro', codigo], queryFn: () => api.lastro.cadeia(codigo ?? ''), enabled: codigo !== null, retry: false });
  const c = cadeia.data;
  return (
    <>
      <CabecalhoDaTela titulo="Lastro" complemento={<span className="t14 mute">Qualquer código leva à cadeia completa</span>}>
        <form
          className="row"
          style={{ gap: 8, width: 460 }}
          onSubmit={(ev) => {
            ev.preventDefault();
            if (texto.trim().length >= 3) setCodigo(texto.trim());
          }}
        >
          <div style={{ flex: 1 }}>
            <Busca rotulo="Buscar por qualquer código" valor={texto} aoMudar={setTexto} placeholder="ATD, accession, etiqueta, guia, laudo, StudyUID, recibo" />
          </div>
          <Botao type="submit" variante="principal">
            Abrir cadeia
          </Botao>
        </form>
      </CabecalhoDaTela>
      {codigo === null ? (
        <section className="card pane fill">
          <Vazio titulo="Digite um código e abra a cadeia." detalhe="A busca fica registrada no lastro com o seu nome." />
        </section>
      ) : (
        <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
          <section className="card pane" aria-label="Cadeia do atendimento">
            <div className="scroll col" style={{ gap: 10 }}>
              <EstadoDaConsulta
                oQue="a cadeia"
                carregando={cadeia.isPending}
                erro={cadeia.error}
                aoTentarDeNovo={() => void cadeia.refetch()}
                semPermissao={{ titulo: 'Você não abre o lastro', detalhe: 'O explorador é da gerência e do auditor.' }}
              />
              {c !== undefined && (
                <>
                  <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{c.paciente.nome}</h2>
                  <div className="t14 num">
                    Nascimento <b>{formatarData(c.paciente.nascimento)}</b>, atendimento {c.atendimento_codigo}
                  </div>
                  {c.integra ? (
                    <Aviso nivel="informativo" titulo="Cadeia íntegra.">
                      Todos os {c.eventos.length} eventos conferem com o hash e com o evento anterior.
                    </Aviso>
                  ) : (
                    <Aviso nivel="bloqueante" titulo="Há evento que não confere com o hash.">
                      Um registro pode ter sido alterado por fora do sistema. Nada foi corrigido aqui. Avise a TI e o encarregado de dados.
                    </Aviso>
                  )}
                  {c.nos.map((n) => (
                    <div key={n.id} className="li" style={{ padding: '8px 0' }}>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div className="t14 b">{NOMES[n.tipo] ?? n.tipo}</div>
                        <div className="t13 mute num">
                          {n.codigo}, {formatarDataHora(n.criado_em, FUSO_DA_CLINICA)}
                        </div>
                      </div>
                      {typeof n.situacao === 'string' && <Chip cor="cinza" texto={n.situacao.replace(/_/g, ' ')} />}
                    </div>
                  ))}
                </>
              )}
            </div>
          </section>
          <section className="card pane" aria-label="Eventos">
            <div className="scroll col" style={{ gap: 6 }}>
              <p className="t13 mute" style={{ margin: 0 }}>
                Eventos imutáveis, encadeados por hash. Nada é apagado.
              </p>
              {c?.eventos.map((e) => (
                <div key={e.sequencia} className="li" style={{ padding: '6px 0' }}>
                  <span className="t13 num mute" style={{ width: 120, flex: 'none' }}>
                    {formatarDataHora(e.criado_em, FUSO_DA_CLINICA)}
                  </span>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div className="t14">
                      {e.tipo.replace(/_/g, ' ')}
                      {typeof e.registro_codigo === 'string' ? `, ${e.registro_codigo}` : ''}
                    </div>
                    <div className="t12 mute num">
                      {e.dominio}, origem {e.origem}, hash {e.hash}
                    </div>
                  </div>
                  {e.integro ? <Chip cor="verde" texto="Confere" /> : <Chip cor="rosa" texto="Não confere" />}
                </div>
              ))}
            </div>
          </section>
        </div>
      )}
    </>
  );
}
