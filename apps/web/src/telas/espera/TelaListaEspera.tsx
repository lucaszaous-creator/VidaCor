import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { PacienteResumo, PedidoDeEspera, PedidoInclusao, PreferenciaDeHorario, PrioridadeDeEspera, TipoDeBloqueio } from '@vidacor/api-client';
import { Abas, Aviso, Botao, Campo, CampoDeData, Chip, Escolha, formatarData, formatarDataHora, formatarHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { EscolhaDePaciente } from '../../componentes/EscolhaDePaciente';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSalas } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const PRIORIDADES: { valor: PrioridadeDeEspera; rotulo: string }[] = [
  { valor: 'rotina', rotulo: 'Rotina' },
  { valor: 'gestante', rotulo: 'Gestante' },
  { valor: 'oncologico', rotulo: 'Oncológico' },
  { valor: 'urgente', rotulo: 'Urgente (pede justificativa do médico)' },
];
const PREFERENCIAS: { valor: PreferenciaDeHorario; rotulo: string }[] = [
  { valor: 'qualquer', rotulo: 'Qualquer horário' },
  { valor: 'manha', rotulo: 'Manhã' },
  { valor: 'tarde', rotulo: 'Tarde' },
];
const TIPOS: { valor: TipoDeBloqueio; rotulo: string }[] = [
  { valor: 'manutencao', rotulo: 'Manutenção' },
  { valor: 'feriado', rotulo: 'Feriado' },
  { valor: 'outro', rotulo: 'Outro' },
];
const COR_DA_PRIORIDADE: Record<string, 'rosa' | 'violeta' | 'azul' | 'cinza'> = { urgente: 'rosa', oncologico: 'violeta', gestante: 'azul', rotina: 'cinza' };
const NOME_DA_PRIORIDADE: Record<string, string> = { urgente: 'Urgente', oncologico: 'Oncológico', gestante: 'Gestante', rotina: 'Rotina' };
const HORA = /^([01]\d|2[0-3]):[0-5]\d$/;

/** Data e hora de Brasília em instante ISO (o Brasil não tem horário de verão desde 2019). */
function instante(data: CalendarDate, hora: string): string {
  return `${data.toString()}T${hora}:00-03:00`;
}

function Pedido({ p }: { p: PedidoDeEspera }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [nome, setNome] = useState('');
  const [nascimento, setNascimento] = useState<CalendarDate | null>(null);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState<'aceite' | 'recusa' | null>(null);
  const atualizar = () => cliente.invalidateQueries({ queryKey: ['lista-espera'] });
  const aceite = useOperacao((_: undefined, chave: string) => api.espera.aceitar(p.oferta?.id ?? '', nome.trim(), nascimento?.toString() ?? '', { idempotencyKey: chave }), { aoConcluir: atualizar });
  const recusa = useOperacao((_: undefined, chave: string) => api.espera.recusar(p.oferta?.id ?? '', motivo.trim(), { idempotencyKey: chave }), { aoConcluir: atualizar });
  const o = p.oferta;
  return (
    <div className="col" style={{ gap: 12 }}>
      <div>
        <h2 className="t15 b" style={{ margin: 0, overflowWrap: 'anywhere' }}>
          {p.paciente.nome}, nascimento {formatarData(p.paciente.nascimento)}
        </h2>
        <span className="t13 mute">
          {p.exame.nome}, na espera desde {formatarData(p.desde.slice(0, 10))}
          {p.justificativa === null || p.justificativa === undefined ? '' : `. Justificativa: ${p.justificativa} (${p.medico_solicitante ?? ''})`}
        </span>
      </div>
      {aceite.resultado !== null && (
        <Aviso nivel="feito" titulo={`Encaixado: ${aceite.resultado.paciente.nome}, ${aceite.resultado.sala.nome} às ${formatarHora(aceite.resultado.inicio, FUSO_DA_CLINICA)}.`}>
          Agendamento {aceite.resultado.agendamento_codigo ?? ''} criado e registrado no lastro. Saiu da lista de espera.
        </Aviso>
      )}
      {recusa.resultado !== null && <Aviso nivel="informativo" titulo="Recusa registrada.">A vaga volta a ser oferecida ao próximo da lista.</Aviso>}
      {o === null || o === undefined ? (
        aceite.resultado === null && <Vazio titulo="Ainda sem vaga oferecida." detalhe="Quando um horário abrir, o sistema oferece pela ordem da fila e avisa pelo WhatsApp." />
      ) : (
        aceite.resultado === null &&
        recusa.resultado === null && (
          <>
            <Aviso nivel="atencao" titulo={`Vaga oferecida: ${o.sala_nome}, ${formatarDataHora(o.inicio, FUSO_DA_CLINICA)}.`}>
              O horário fica reservado até {formatarHora(o.expira_em, FUSO_DA_CLINICA)}. Sem resposta, vai para o próximo da lista.
            </Aviso>
            <p className="t14" style={{ margin: 0 }}>
              Para encaixar, confira com o paciente o nome completo e o nascimento.
            </p>
            <div className="col" style={{ gap: 10 }}>
              <Campo rotulo="Nome dito pelo paciente" value={nome} onChange={setNome} erro={tentou === 'aceite' && nome.trim().length < 3 ? 'Escreva o nome completo.' : null} />
              <CampoDeData rotulo="Nascimento dito pelo paciente" valor={nascimento} aoMudar={setNascimento} erro={tentou === 'aceite' && nascimento === null ? 'Digite o nascimento.' : null} />
            </div>
            {aceite.erro !== null && <ErroDoAto erro={aceite.erro} ato="encaixar o paciente" />}
            <div className="row wrap" style={{ gap: 8, minWidth: 0 }}>
              <Botao
                variante="principal"
                ocupado={aceite.enviando}
                onPress={() => {
                  setTentou('aceite');
                  if (nome.trim().length >= 3 && nascimento !== null) aceite.enviar(undefined);
                }}
              >
                <span style={{ whiteSpace: 'normal', textAlign: 'left' }}>Encaixar {p.paciente.nome}</span>
              </Botao>
            </div>
            <Campo rotulo="Motivo da recusa" value={motivo} onChange={setMotivo} erro={tentou === 'recusa' && motivo.trim().length < 3 ? 'Escreva o motivo.' : null} />
            {recusa.erro !== null && <ErroDoAto erro={recusa.erro} ato="registrar a recusa" />}
            <div className="row">
              <Botao
                ocupado={recusa.enviando}
                onPress={() => {
                  setTentou('recusa');
                  if (motivo.trim().length >= 3) recusa.enviar(undefined);
                }}
              >
                Registrar recusa
              </Botao>
            </div>
          </>
        )
      )}
    </div>
  );
}

function Inclusao() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const catalogo = useQuery({ queryKey: ['orcamento-catalogo'], queryFn: () => api.orcamentos.catalogo() });
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [exame, setExame] = useState('');
  const [prioridade, setPrioridade] = useState<string>('rotina');
  const [preferencia, setPreferencia] = useState<string>('qualquer');
  const [justificativa, setJustificativa] = useState('');
  const [medico, setMedico] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: PedidoInclusao, chave: string) => api.espera.incluir(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['lista-espera'] }),
  });
  const urgente = prioridade === 'urgente';
  const ok = paciente !== null && exame !== '' && (!urgente || (justificativa.trim().length >= 5 && medico.trim().length >= 3));
  if (op.resultado !== null) {
    return (
      <Aviso nivel="feito" titulo={`${op.resultado.paciente.nome} entrou na lista de espera.`} acoes={<Botao pequeno onPress={op.reiniciar}>Incluir outro</Botao>}>
        Posição {op.resultado.posicao ?? '-'} pela ordem do servidor (prioridade e tempo de espera).
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Incluir paciente na espera"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok)
          op.enviar({
            paciente_id: paciente.id,
            exame_tuss: exame,
            prioridade: prioridade as PrioridadeDeEspera,
            preferencia: preferencia as PreferenciaDeHorario,
            justificativa: urgente ? justificativa.trim() : null,
            medico_solicitante: urgente ? medico.trim() : null,
          });
      }}
    >
      <EscolhaDePaciente rotulo="Paciente" paciente={paciente} aoEscolher={setPaciente} erro={tentou && paciente === null ? 'Escolha o paciente.' : null} />
      <Escolha rotulo="Exame" valor={exame} aoMudar={setExame} vazio="Escolha o exame" opcoes={(catalogo.data?.exames ?? []).map((e) => ({ valor: e.tuss, rotulo: e.nome }))} erro={tentou && exame === '' ? 'Escolha o exame.' : null} />
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Escolha rotulo="Prioridade clínica" valor={prioridade} aoMudar={setPrioridade} opcoes={PRIORIDADES} />
        <Escolha rotulo="Preferência de horário" valor={preferencia} aoMudar={setPreferencia} opcoes={PREFERENCIAS} />
      </div>
      {urgente && (
        <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.4fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
          <Campo rotulo="Justificativa" value={justificativa} onChange={setJustificativa} erro={tentou && justificativa.trim().length < 5 ? 'Urgente pede a justificativa.' : null} />
          <Campo rotulo="Médico solicitante" value={medico} onChange={setMedico} erro={tentou && medico.trim().length < 3 ? 'Diga quem pediu.' : null} />
        </div>
      )}
      <p className="t13 mute" style={{ margin: 0 }}>
        A posição na fila é calculada pelo servidor.
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="incluir na lista de espera" />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Incluir na espera
        </Botao>
      </div>
    </form>
  );
}

function Bloqueio() {
  const { api } = useDependencias();
  const salas = useQuery(consultaSalas(api));
  const [sala, setSala] = useState('');
  const [tipo, setTipo] = useState<string>('manutencao');
  const [data, setData] = useState<CalendarDate | null>(null);
  const [de, setDe] = useState('');
  const [ate, setAte] = useState('');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((_: undefined, chave: string) =>
    api.bloqueios.criar({ sala_ids: [sala], tipo: tipo as TipoDeBloqueio, inicio: instante(data as CalendarDate, de), fim: instante(data as CalendarDate, ate), motivo: motivo.trim() }, { idempotencyKey: chave }),
  );
  const horasOk = HORA.test(de) && HORA.test(ate) && de < ate;
  const ok = sala !== '' && data !== null && horasOk && motivo.trim().length >= 5;
  if (op.resultado !== null) {
    const b = op.resultado[0];
    return (
      <Aviso nivel="feito" titulo={`Horário bloqueado: ${b?.sala.nome ?? ''}.`} acoes={<Botao pequeno onPress={op.reiniciar}>Bloquear outro</Botao>}>
        Ninguém agenda nesse período. Para liberar, retire o bloqueio com motivo; a vaga vai para a lista de espera.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Bloquear horário"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok) op.enviar(undefined);
      }}
    >
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Escolha rotulo="Sala" valor={sala} aoMudar={setSala} vazio="Escolha a sala" opcoes={(salas.data ?? []).map((s) => ({ valor: s.id, rotulo: s.nome }))} erro={tentou && sala === '' ? 'Escolha a sala.' : null} />
        <Escolha rotulo="Tipo" valor={tipo} aoMudar={setTipo} opcoes={TIPOS} />
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.4fr) minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <CampoDeData rotulo="Dia" valor={data} aoMudar={setData} erro={tentou && data === null ? 'Digite o dia.' : null} />
        <Campo rotulo="De (hh:mm)" value={de} onChange={setDe} erro={tentou && !horasOk ? 'Use hh:mm, início antes do fim.' : null} />
        <Campo rotulo="Até (hh:mm)" value={ate} onChange={setAte} />
      </div>
      <Campo rotulo="Motivo do bloqueio" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Escreva o motivo (5 letras ou mais).' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="bloquear o horário" />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Bloquear horário
        </Botao>
      </div>
    </form>
  );
}

/**
 * Lista de espera (Lista-Espera.dc.html): fila pela ordem do servidor, vaga oferecida com prazo e
 * encaixe só com nome e nascimento conferidos; inclusão na espera e bloqueio de horário.
 */
export function TelaListaEspera() {
  const { api } = useDependencias();
  const fila = useQuery({ queryKey: ['lista-espera'], queryFn: () => api.espera.listar(), refetchInterval: 30_000 });
  const [escolhido, setEscolhido] = useState<string | null>(null);
  const [aba, setAba] = useState('incluir');
  const atual = fila.data?.find((p) => p.id === escolhido) ?? null;
  const ofertados = fila.data?.filter((p) => p.oferta !== null && p.oferta !== undefined).length ?? 0;
  return (
    <>
      <CabecalhoDaTela
        titulo="Lista de espera"
        complemento={fila.data === undefined ? undefined : <span className="t14 mute">{fila.data.length} aguardando vaga, {ofertados} com oferta aguardando resposta</span>}
      />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.1fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Aguardando vaga" style={{ padding: 0 }}>
          <div className="card-h row" style={{ padding: '14px 16px 0', marginBottom: 8 }}>
            <h2 style={{ flex: 1 }}>Aguardando vaga</h2>
            <span className="t13 mute">Por prioridade clínica e tempo de espera</span>
          </div>
          <div className="scroll">
            <EstadoDaConsulta oQue="a lista de espera" carregando={fila.isPending} erro={fila.error} aoTentarDeNovo={() => void fila.refetch()} />
            {fila.data?.length === 0 && <Vazio titulo="Ninguém na lista de espera." />}
            <ul style={{ listStyle: 'none', margin: 0, padding: '0 8px 8px' }}>
              {fila.data?.map((p) => (
                <li key={p.id}>
                  <button
                    type="button"
                    aria-current={p.id === escolhido ? 'true' : undefined}
                    style={{ width: '100%', textAlign: 'left', display: 'flex', gap: 10, alignItems: 'center', padding: '10px 8px', border: 0, background: p.id === escolhido ? 'var(--line)' : 'transparent', borderBottom: '1px solid var(--line)', cursor: 'pointer', font: 'inherit', color: 'inherit' }}
                    onClick={() => {
                      setEscolhido(p.id);
                    }}
                  >
                    <span className="num b" style={{ width: 28 }}>
                      {p.posicao ?? ''}
                    </span>
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <b style={{ display: 'block' }}>
                        {p.paciente.nome}, nascimento {formatarData(p.paciente.nascimento)}
                      </b>
                      <span className="t13 mute">
                        {p.exame.nome}
                        {p.oferta === null || p.oferta === undefined ? '' : `, vaga oferecida até ${formatarHora(p.oferta.expira_em, FUSO_DA_CLINICA)}`}
                      </span>
                    </span>
                    <Chip cor={COR_DA_PRIORIDADE[p.prioridade] ?? 'cinza'} texto={NOME_DA_PRIORIDADE[p.prioridade] ?? p.prioridade} />
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label={atual === null ? 'Incluir ou bloquear' : 'Pedido de espera'}>
          <div className="scroll col" style={{ gap: 12 }}>
            {atual !== null ? (
              <>
                <div className="row">
                  <Botao
                    pequeno
                    onPress={() => {
                      setEscolhido(null);
                    }}
                  >
                    Voltar para incluir ou bloquear
                  </Botao>
                </div>
                <Pedido key={atual.id} p={atual} />
              </>
            ) : (
              <Abas
                rotulo="O que fazer"
                selecionada={aba}
                aoSelecionar={setAba}
                abas={[
                  { id: 'incluir', rotulo: 'Incluir na espera', conteudo: <Inclusao /> },
                  { id: 'bloquear', rotulo: 'Bloquear horário', conteudo: <Bloqueio /> },
                ]}
              />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
