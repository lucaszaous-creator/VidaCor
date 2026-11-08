import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ErroDaApi, type AgendamentoCriado, type Guia, type PacienteResumo, type PedidoAgendamento } from '@vidacor/api-client';
import {
  Aviso,
  Botao,
  Busca,
  CampoDeData,
  Escolha,
  formatarData,
  formatarDataHora,
  formatarHora,
  Vazio,
} from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { chaves, consultaHorariosLivres, consultaPacientes, consultaSalas } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { PedidoDeAutorizacao } from '../autorizacoes/PedidoDeAutorizacao';
import { GuiaRegistrada, ProcedimentosDoPedido, RegistroDaGuia, tussValidos, type LinhaDeProcedimento } from './SecaoDaGuia';

function textoDoCampo(erro: unknown, campo: string): string | null {
  if (!(erro instanceof ErroDaApi)) return null;
  const v = erro.campo(campo);
  return typeof v === 'string' ? v : null;
}

function Sucesso({ a, aoNovo }: { a: AgendamentoCriado; aoNovo: () => void }) {
  const quando = `${formatarDataHora(a.inicio, FUSO_DA_CLINICA)}, ${a.sala.nome}`;
  return a.situacao === 'reserva_provisoria' ? (
    <Aviso nivel="atencao" titulo="Reserva provisória feita.">
      {a.paciente.nome}, nascimento {formatarData(a.paciente.nascimento)}, {quando}. O horário fica reservado até{' '}
      <b className="num">{a.expira_em === null || a.expira_em === undefined ? 'o prazo do servidor' : formatarDataHora(a.expira_em, FUSO_DA_CLINICA)}</b>. Registre a
      guia antes do prazo; sem ela, a reserva expira, o horário volta a ficar livre e o paciente é avisado.
    </Aviso>
  ) : (
    <Aviso
      nivel="feito"
      titulo="Agendamento confirmado."
      acoes={
        <Botao pequeno onPress={aoNovo}>
          Novo agendamento
        </Botao>
      }
    >
      {a.paciente.nome}, nascimento {formatarData(a.paciente.nascimento)}, {quando}. Protocolo <b className="num">{a.codigo}</b>. A guia foi consumida
      por este agendamento.
    </Aviso>
  );
}

/**
 * Novo agendamento de convênio com guia (Agendamento-Guia.dc.html, FC-AGD). Horário ocupado não
 * aparece como livre (R1); sem guia, só reserva provisória com prazo. Saldo, validade e autorização
 * quem confere é o servidor.
 */
export function TelaNovoAgendamento() {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const [texto, setTexto] = useState('');
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [salaId, setSalaId] = useState('');
  const [data, setData] = useState<CalendarDate | null>(null);
  const [inicio, setInicio] = useState<string | null>(null);
  const [linhas, setLinhas] = useState<LinhaDeProcedimento[]>([{ chave: 1, tuss: '', quantidade: '1' }]);
  const [guia, setGuia] = useState<Guia | null>(null);
  const [escolhidos, setEscolhidos] = useState<string[]>([]);
  const [tentou, setTentou] = useState(false);
  const [versaoDaGuia, setVersaoDaGuia] = useState(0);

  const salas = useQuery(consultaSalas(api));
  const busca = texto.trim();
  const pacientes = useQuery({ ...consultaPacientes(api, busca), enabled: busca.length >= 3 && paciente === null });
  const dia = data?.toString() ?? '';
  const horarios = useQuery({ ...consultaHorariosLivres(api, salaId, dia), enabled: salaId !== '' && dia !== '' });

  const agendar = useOperacao((p: PedidoAgendamento, chave: string) => api.agendamentos.criar(p, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      await queryClient.invalidateQueries({ queryKey: chaves.agendaTodas });
      await queryClient.invalidateQueries({ queryKey: chaves.horariosLivres(salaId, dia) });
    },
  });
  const confirmar = useOperacao((g: { agendamento: string; guia: string }, chave: string) =>
    api.agendamentos.vincularGuia(g.agendamento, g.guia, { idempotencyKey: chave }),
  );

  const criado = confirmar.resultado ?? agendar.resultado;
  const procedimentos = guia === null ? tussValidos(linhas) : escolhidos;
  const pronto = paciente !== null && salaId !== '' && inicio !== null && procedimentos.length > 0;

  function enviar(reserva: boolean): void {
    setTentou(true);
    if (!pronto) return;
    agendar.enviar({
      paciente_id: paciente.id,
      sala_id: salaId,
      inicio,
      procedimentos,
      guia_id: reserva ? null : (guia?.id ?? null),
      reserva_provisoria: reserva,
    });
  }

  function novo(): void {
    agendar.reiniciar();
    confirmar.reiniciar();
    setPaciente(null);
    setTexto('');
    setInicio(null);
    setGuia(null);
    setEscolhidos([]);
    setTentou(false);
  }

  const erro = agendar.erro;
  const codigoDoErro = erro instanceof ErroDaApi ? erro.codigo : '';
  const sugestoes = erro instanceof ErroDaApi && codigoDoErro === 'horario-ocupado' ? erro.campo('sugestoes') : null;
  const listaDeSugestoes = Array.isArray(sugestoes) ? sugestoes.filter((s): s is string => typeof s === 'string') : [];
  const atendimentoDaGuia = textoDoCampo(erro, 'atendimento_codigo');
  const tussRecusado = textoDoCampo(erro, 'tuss');
  const validade = textoDoCampo(erro, 'validade');

  let complemento: string | undefined;
  if (codigoDoErro === 'guia-em-uso' && atendimentoDaGuia !== null) complemento = `Guia em uso no atendimento ${atendimentoDaGuia}: registre outra guia.`;
  if (codigoDoErro === 'procedimento-nao-autorizado' && tussRecusado !== null) complemento = `Procedimento TUSS ${tussRecusado} fora da guia: peça autorização à operadora.`;
  if (codigoDoErro === 'senha-vencida' && validade !== null) complemento = `A senha vale até ${formatarData(validade)}: remarque antes disso ou peça prorrogação.`;

  return (
    <>
      <CabecalhoDaTela titulo="Novo agendamento" complemento={<span className="t14 mute">Convênio com guia de autorização</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Paciente e horário">
          <div className="scroll col" style={{ gap: 14 }}>
            <h2 className="t15 b">Paciente</h2>
            {paciente === null ? (
              <>
                <Busca rotulo="Buscar paciente por nome" valor={texto} aoMudar={setTexto} placeholder="Nome do paciente (mínimo 3 letras)" />
                {tentou && <span className="campo-erro">Escolha o paciente.</span>}
                {busca.length >= 3 && (
                  <div>
                    <EstadoDaConsulta oQue="os pacientes" carregando={pacientes.isPending} erro={pacientes.error} aoTentarDeNovo={() => void pacientes.refetch()} linhas={2} />
                    {pacientes.data?.length === 0 && <Vazio titulo="Nenhum paciente com esse nome" detalhe="Confira a grafia ou busque pelo sobrenome." />}
                    {pacientes.data?.map((p) => (
                      <LinhaDoPaciente
                        key={p.id}
                        nome={p.nome}
                        nascimento={p.nascimento}
                        selecionada={false}
                        aoAbrir={() => {
                          setPaciente(p);
                        }}
                      />
                    ))}
                  </div>
                )}
              </>
            ) : (
              <div className="row" style={{ gap: 10 }}>
                <span style={{ flex: 1 }}>
                  <b className="t15">{paciente.nome}</b>
                  <span className="t14 num" style={{ display: 'block', color: 'var(--ink2)' }}>
                    Nascimento <b>{formatarData(paciente.nascimento)}</b>
                  </span>
                </span>
                <Botao pequeno onPress={() => { setPaciente(null); setGuia(null); }} isDisabled={criado !== null}>
                  Trocar paciente
                </Botao>
              </div>
            )}

            <h2 className="t15 b">Sala e horário</h2>
            <div className="row wrap" style={{ alignItems: 'flex-start' }}>
              <div style={{ flex: '1 1 200px' }}>
                <Escolha
                  rotulo="Sala"
                  valor={salaId}
                  aoMudar={(v) => {
                    setSalaId(v);
                    setInicio(null);
                  }}
                  vazio="Escolha a sala"
                  opcoes={(salas.data ?? []).map((s) => ({ valor: s.id, rotulo: s.nome }))}
                  erro={tentou && salaId === '' ? 'Escolha a sala.' : null}
                />
              </div>
              <div style={{ flex: '1 1 200px' }}>
                <CampoDeData
                  rotulo="Dia do exame"
                  valor={data}
                  aoMudar={(d) => {
                    setData(d);
                    setInicio(null);
                  }}
                  erro={tentou && data === null ? 'Digite o dia do exame.' : null}
                />
              </div>
            </div>
            {salaId !== '' && dia !== '' && (
              <div className="col" style={{ gap: 8 }}>
                <span className="t13 b mute">Horários livres</span>
                <EstadoDaConsulta oQue="os horários livres" carregando={horarios.isPending} erro={horarios.error} aoTentarDeNovo={() => void horarios.refetch()} linhas={1} />
                {horarios.data?.length === 0 && <Vazio titulo="Nenhum horário livre neste dia" detalhe="Escolha outro dia ou outra sala." />}
                <div className="row wrap" style={{ gap: 8 }} role="group" aria-label="Horários livres">
                  {horarios.data?.map((h) => (
                    <button
                      key={h}
                      type="button"
                      className="slot-free"
                      style={{ minWidth: 88, padding: '0 12px', ...(h === inicio ? { background: 'var(--navy)', color: '#fff', borderStyle: 'solid' } : {}) }}
                      aria-pressed={h === inicio}
                      onClick={() => {
                        setInicio(h);
                      }}
                    >
                      {formatarHora(h, FUSO_DA_CLINICA)}
                    </button>
                  ))}
                </div>
              </div>
            )}
            {tentou && inicio === null && <span className="campo-erro">Escolha um horário livre.</span>}

            <h2 className="t15 b">Procedimentos do pedido</h2>
            <ProcedimentosDoPedido linhas={linhas} aoMudar={setLinhas} tentou={tentou && guia === null} desabilitado={guia !== null} />
          </div>
        </section>

        <section className="card pane" aria-label="Convênio e guia">
          <div className="scroll col" style={{ gap: 14 }}>
            <h2 className="t15 b">Guia de autorização</h2>
            {guia === null ? (
              <RegistroDaGuia
                key={versaoDaGuia}
                pacienteId={paciente?.id ?? null}
                linhas={linhas}
                aoRegistrar={(g) => {
                  setGuia(g);
                  setEscolhidos(g.procedimentos.map((p) => p.tuss));
                }}
              />
            ) : (
              <GuiaRegistrada
                guia={guia}
                escolhidos={escolhidos}
                aoEscolher={setEscolhidos}
                aoTrocar={() => {
                  setGuia(null);
                  setEscolhidos([]);
                  setVersaoDaGuia((v) => v + 1);
                }}
              />
            )}

            <div aria-live="polite" className="col" style={{ gap: 10 }}>
              {criado !== null && <Sucesso a={criado} aoNovo={novo} />}
              {criado?.situacao === 'reserva_provisoria' && confirmar.resultado === null && guia === null && <PedidoDeAutorizacao agendamentoId={criado.id} />}
              {erro !== null && (
                <ErroDoAto
                  erro={erro}
                  ato="agendar"
                  aoTentarDeNovo={() => {
                    enviar(guia === null);
                  }}
                  complemento={complemento}
                  acoes={
                    listaDeSugestoes.length > 0 ? (
                      <div className="row wrap" style={{ gap: 8 }} role="group" aria-label="Próximos horários livres">
                        {listaDeSugestoes.map((s) => (
                          <Botao
                            key={s}
                            pequeno
                            onPress={() => {
                              setInicio(s);
                              agendar.limparErro();
                            }}
                          >
                            {`Usar ${formatarHora(s, FUSO_DA_CLINICA)}`}
                          </Botao>
                        ))}
                      </div>
                    ) : codigoDoErro === 'guia-em-uso' ? (
                      <Botao
                        pequeno
                        onPress={() => {
                          setGuia(null);
                          setVersaoDaGuia((v) => v + 1);
                          agendar.limparErro();
                        }}
                      >
                        Registrar outra guia
                      </Botao>
                    ) : undefined
                  }
                />
              )}
              {confirmar.erro !== null && <ErroDoAto erro={confirmar.erro} ato="confirmar a reserva com a guia" />}
            </div>
          </div>
          <div className="row wrap" style={{ gap: 8, paddingTop: 12, borderTop: '1px solid var(--line)' }}>
            {agendar.resultado?.situacao === 'reserva_provisoria' && confirmar.resultado === null ? (
              <Botao
                variante="principal"
                isDisabled={guia === null}
                ocupado={confirmar.enviando}
                onPress={() => {
                  if (guia !== null) confirmar.enviar({ agendamento: agendar.resultado?.id ?? '', guia: guia.id });
                }}
              >
                Confirmar a reserva com a guia
              </Botao>
            ) : (
              <>
                <Botao variante="principal" onPress={() => { enviar(false); }} ocupado={agendar.enviando} isDisabled={guia === null || criado !== null}>
                  Agendar com a guia
                </Botao>
                <Botao onPress={() => { enviar(true); }} isDisabled={guia !== null || agendar.enviando || criado !== null}>
                  Reservar sem guia
                </Botao>
              </>
            )}
          </div>
        </section>
      </div>
    </>
  );
}
