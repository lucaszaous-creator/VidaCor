import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi, Link } from '@tanstack/react-router';
import { ErroDaApi, type ResultadoDoBipe } from '@vidacor/api-client';
import { Aviso, BarraDoPaciente, Botao, Escolha, formatarData, formatarHora, Icone, Vazio } from '@vidacor/design-system';
import { useId, useRef, useState, type FormEvent } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { chaves, consultaFilaDaSala, consultaSalas } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { aparenciaDoAtendimento } from '../agenda/situacoes';

const rota = getRouteApi('/app/sala');

const TRIAGEM: Record<string, string> = {
  liberado: 'triagem liberada',
  liberado_com_ressalvas: 'triagem liberada com ressalvas',
  precisa_aprovacao_medica: 'triagem aguarda a conduta médica',
};

function salaCorreta(erro: unknown): { id: string; nome: string } | null {
  if (!(erro instanceof ErroDaApi) || erro.codigo !== 'paciente-de-outra-sala') {
    return null;
  }
  const s = erro.campo('sala_correta') as { id?: unknown; nome?: unknown } | undefined;
  return typeof s?.id === 'string' && typeof s.nome === 'string' ? { id: s.id, nome: s.nome } : null;
}

function ResultadoLiberado({ r }: { r: ResultadoDoBipe }) {
  return r.worklist === 'enviada' ? (
    <Aviso nivel="feito" titulo="Exame liberado.">
      Lista de trabalho enviada ao {r.aparelho.nome} ({r.aparelho.ae_title}) com {r.atendimento.paciente.nome}, nascimento{' '}
      {formatarData(r.atendimento.paciente.nascimento)}. Ninguém digita nome no aparelho.
    </Aviso>
  ) : (
    <Aviso nivel="informativo" titulo="A lista de trabalho já estava no aparelho.">
      {r.atendimento.paciente.nome} já está no {r.aparelho.nome}. Nada foi enviado de novo.
    </Aviso>
  );
}

/**
 * Sala de exame (Sala-Exame.dc.html): bipe da pulseira (leitor de código de barras = teclado + Enter),
 * fila do dia da sala e o paciente em foco. Quem decide se o exame é liberado é o servidor (FC-PUL).
 */
export function TelaSala() {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const busca = rota.useSearch();
  const navegar = rota.useNavigate();
  const idCampo = useId();
  const campo = useRef<HTMLInputElement>(null);
  const [codigo, setCodigo] = useState('');
  const [ultimoCodigo, setUltimoCodigo] = useState('');
  const [emFoco, setEmFoco] = useState<ResultadoDoBipe | null>(null);
  const salas = useQuery(consultaSalas(api));
  const salaId = busca.sala ?? salas.data?.[0]?.id;
  const sala = salas.data?.find((s) => s.id === salaId);
  const fila = useQuery({ ...consultaFilaDaSala(api, salaId ?? ''), enabled: salaId !== undefined });

  const bipe = useOperacao((c: string, chave: string) => api.salas.bipar(salaId ?? '', { codigo_etiqueta: c }, { idempotencyKey: chave }), {
    aoConcluir: async (r) => {
      setEmFoco(r);
      await queryClient.invalidateQueries({ queryKey: chaves.filaDaSala(salaId ?? '') });
    },
  });

  function bipar(ev: FormEvent): void {
    ev.preventDefault();
    const c = codigo.trim().toUpperCase();
    if (c === '' || salaId === undefined || bipe.enviando) {
      return;
    }
    // Cada bipe é uma operação nova; a repetição do mesmo bipe o servidor responde "já enviada".
    bipe.reiniciar();
    setUltimoCodigo(c);
    bipe.enviar(c);
    setCodigo('');
    campo.current?.focus();
  }

  const outra = salaCorreta(bipe.erro);
  const paciente = emFoco?.atendimento.paciente;

  return (
    <>
      <CabecalhoDaTela titulo={sala?.nome ?? 'Sala de exame'}>
        <div style={{ width: 260 }}>
          <Escolha
            rotulo="Sala"
            valor={salaId ?? ''}
            aoMudar={(v) => {
              setEmFoco(null);
              bipe.reiniciar();
              void navegar({ search: { sala: v } });
            }}
            opcoes={(salas.data ?? []).map((s) => ({ valor: s.id, rotulo: s.nome }))}
          />
        </div>
      </CabecalhoDaTela>

      {paciente === undefined ? (
        <Aviso nivel="informativo" titulo="Nenhum paciente em foco.">
          Bipe a pulseira do paciente para liberar o exame e enviar a lista de trabalho ao aparelho.
        </Aviso>
      ) : (
        <BarraDoPaciente
          nome={paciente.nome}
          nascimento={formatarData(paciente.nascimento)}
          resumo={`${emFoco?.atendimento.codigo ?? ''}, accession ${emFoco?.atendimento.accession ?? ''}, ${sala?.nome ?? ''}`}
        />
      )}

      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Bipe da pulseira">
          <div className="card-h">
            <h2>Bipe da pulseira</h2>
          </div>
          <div className="scroll col" style={{ gap: 14 }}>
            <form className="col" style={{ gap: 10 }} onSubmit={bipar} noValidate>
              <label className="field" htmlFor={idCampo}>
                Código da pulseira
                <span className="search" style={{ minHeight: 52 }}>
                  <Icone nome="leitor" />
                  <input
                    id={idCampo}
                    ref={campo}
                    value={codigo}
                    onChange={(e) => {
                      setCodigo(e.target.value);
                    }}
                    autoComplete="off"
                    spellCheck={false}
                    // Leitor de código de barras digita e manda Enter: o campo já vem com o foco.
                    autoFocus
                    placeholder="Bipe a pulseira ou digite o código"
                    aria-describedby={`${idCampo}-ajuda`}
                  />
                </span>
                <span id={`${idCampo}-ajuda`} className="campo-ajuda">
                  O leitor digita o código e confirma sozinho. Sem leitor, digite e aperte Enter.
                </span>
              </label>
              <div className="row">
                <Botao type="submit" variante="principal" ocupado={bipe.enviando} isDisabled={salaId === undefined}>
                  Conferir pulseira
                </Botao>
              </div>
            </form>
            <div aria-live="polite" className="col" style={{ gap: 10 }}>
              {bipe.resultado !== null && <ResultadoLiberado r={bipe.resultado} />}
              {bipe.erro !== null && (
                <ErroDoAto
                  erro={bipe.erro}
                  ato="conferir a pulseira"
                  aoTentarDeNovo={() => {
                    // Mesma operação, mesma chave: o servidor não envia a lista duas vezes.
                    bipe.enviar(ultimoCodigo);
                  }}
                  acoes={
                    outra === null ? undefined : (
                      <Link to="/sala" search={{ sala: outra.id }} className="btn btn-sm">
                        Ver a fila da {outra.nome}
                      </Link>
                    )
                  }
                  complemento={outra === null ? undefined : <b>Paciente da {outra.nome}: encaminhe e bipe a próxima pulseira.</b>}
                />
              )}
            </div>
          </div>
        </section>

        <section className="card pane" aria-label="Fila da sala" style={{ padding: 0 }}>
          <div className="card-h" style={{ padding: '14px 16px 0', marginBottom: 8 }}>
            <h2>Fila de hoje</h2>
          </div>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="a fila da sala"
              carregando={fila.isPending && salaId !== undefined}
              erro={fila.error ?? salas.error}
              aoTentarDeNovo={() => void fila.refetch()}
              semPermissao={{ titulo: 'Você não tem acesso a esta sala', detalhe: 'A fila aparece para quem está lotado na sala hoje.' }}
            />
            {fila.data?.itens.length === 0 && (
              <div style={{ padding: 16 }}>
                <Vazio titulo="Nenhum paciente na fila desta sala hoje" />
              </div>
            )}
            {fila.data?.itens.map((i) => {
              const ap = aparenciaDoAtendimento(i.situacao);
              return (
                <div key={i.atendimento_id} className="row" style={{ gap: 0, paddingRight: 12 }}>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <LinhaDoPaciente
                      nome={i.paciente.nome}
                      nascimento={i.paciente.nascimento}
                      inicio={formatarHora(i.inicio, FUSO_DA_CLINICA)}
                      detalhe={i.exame.nome}
                      situacao={{ texto: ap.texto, cor: ap.cor }}
                      marca={
                        i.requer_contraste
                          ? `Contraste: ${i.triagem_resultado === null ? 'sem triagem' : (TRIAGEM[i.triagem_resultado] ?? i.triagem_resultado)}`
                          : undefined
                      }
                      selecionada={i.paciente.id === paciente?.id}
                      contextoDoMenu={i.exame.nome}
                      aoAbrir={() => {
                        campo.current?.focus();
                      }}
                    />
                  </div>
                  {i.requer_contraste && (
                    <Link
                      to="/triagem/$atendimentoId"
                      params={{ atendimentoId: i.atendimento_id }}
                      className="btn btn-sm"
                      aria-label={`Triagem de ${i.paciente.nome}`}
                    >
                      Triagem
                    </Link>
                  )}
                </div>
              );
            })}
          </div>
        </section>
      </div>
    </>
  );
}
