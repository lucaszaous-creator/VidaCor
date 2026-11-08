import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import type { EstudoPendente, PedidoConciliacao } from '@vidacor/api-client';
import { Aviso, Botao, Campo, CampoDeData, Dialogo, Escolha, formatarData, formatarDataHora, formatarHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaAgenda } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA, hojeNaClinica } from '../agenda/grade';

const rota = getRouteApi('/app/estudos');
const CHAVE = ['estudos', 'pendentes'] as const;

function Conciliacao({ estudo, data, aoConcluir }: { estudo: EstudoPendente; data: string; aoConcluir: () => void }) {
  const { api } = useDependencias();
  const agenda = useQuery(consultaAgenda(api, data, undefined));
  const [atendimentoId, setAtendimentoId] = useState('');
  const [nome, setNome] = useState('');
  const [nascimento, setNascimento] = useState<CalendarDate | null>(null);
  const [tentou, setTentou] = useState(false);
  const [confirmando, setConfirmando] = useState(false);
  const op = useOperacao((p: PedidoConciliacao, chave: string) => api.estudos.conciliar(estudo.id, p, { idempotencyKey: chave }), {
    aoConcluir: () => {
      setConfirmando(false);
      aoConcluir();
    },
    aoFalhar: () => {
      setConfirmando(false);
    },
  });
  const atendidos = (agenda.data?.itens ?? []).filter((i) => typeof i.atendimento_id === 'string');
  const item = atendidos.find((i) => i.atendimento_id === atendimentoId);

  function conferir(): void {
    setTentou(true);
    if (item !== undefined && nome.trim() !== '' && nascimento !== null) setConfirmando(true);
  }

  return (
    <form
      className="col"
      style={{ gap: 12 }}
      noValidate
      aria-label={`Conciliar o estudo ${estudo.codigo}`}
      onSubmit={(ev) => {
        ev.preventDefault();
        conferir();
      }}
    >
      <h2 className="t15 b">
        Estudo <span className="num">{estudo.codigo}</span>
      </h2>
      <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
        {estudo.pendencia ?? 'Sem paciente conciliado'}. Imagem sem paciente conciliado não vai para laudo. Escolha o atendimento e digite o nome e o
        nascimento como aparecem no estudo: o servidor confere os dois identificadores.
      </p>
      <Escolha
        rotulo="Atendimento do dia"
        valor={atendimentoId}
        aoMudar={setAtendimentoId}
        vazio="Escolha o atendimento"
        opcoes={atendidos.map((i) => ({
          valor: i.atendimento_id ?? '',
          rotulo: `${formatarHora(i.inicio, FUSO_DA_CLINICA)}, ${i.sala.nome}, ${i.exame.nome}, ${i.paciente.nome}`,
        }))}
        erro={tentou && item === undefined ? 'Escolha o atendimento.' : null}
      />
      <Campo rotulo="Nome como está no estudo" value={nome} onChange={setNome} autoComplete="off" erro={tentou && nome.trim() === '' ? 'Digite o nome.' : null} />
      <CampoDeData rotulo="Nascimento como está no estudo" valor={nascimento} aoMudar={setNascimento} erro={tentou && nascimento === null ? 'Digite o nascimento.' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="conciliar o estudo" />}
      <div className="row">
        <Botao type="submit" variante="principal">
          Conferir e conciliar
        </Botao>
      </div>
      {item !== undefined && (
        <Dialogo
          aberto={confirmando || op.enviando}
          titulo={`Vincular o estudo ${estudo.codigo} a ${item.paciente.nome}?`}
          paciente={{ nome: item.paciente.nome, nascimento: formatarData(item.paciente.nascimento) }}
          rotuloConfirmar="Vincular estudo"
          ocupado={op.enviando}
          aoFechar={() => {
            setConfirmando(false);
          }}
          aoConfirmar={() => {
            if (nascimento !== null) op.enviar({ atendimento_id: atendimentoId, nome: nome.trim(), nascimento: nascimento.toString() });
          }}
        >
          <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
            {estudo.imagens} imagens recebidas em {formatarDataHora(estudo.recebido_em, FUSO_DA_CLINICA)} passam a ser deste atendimento e entram na fila de
            laudo.
          </p>
        </Dialogo>
      )}
    </form>
  );
}

/** Conciliação manual de estudos sem paciente (docs/30 § 7, FC-LAU-09, FC-CTG-08), com dois identificadores. */
export function TelaEstudos() {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const busca = rota.useSearch();
  const data = busca.data ?? hojeNaClinica();
  const pendentes = useQuery({ queryKey: CHAVE, queryFn: () => api.estudos.pendentes(), refetchInterval: 60_000 });
  const [selecionado, setSelecionado] = useState<string | null>(null);
  const [feito, setFeito] = useState<string | null>(null);
  const estudo = pendentes.data?.find((e) => e.id === selecionado) ?? null;

  return (
    <>
      <CabecalhoDaTela titulo="Conciliar estudos" complemento={<span className="t14 mute">Estudos recebidos sem paciente conciliado</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gap: 16 }}>
        <section className="card pane" aria-label="Estudos pendentes" style={{ padding: 0 }}>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="os estudos pendentes"
              carregando={pendentes.isPending}
              erro={pendentes.error}
              aoTentarDeNovo={() => void pendentes.refetch()}
              semPermissao={{ titulo: 'Você não concilia estudos', detalhe: 'A conciliação manual é do médico.' }}
            />
            {pendentes.data?.length === 0 && (
              <div style={{ padding: 16 }}>
                <Vazio titulo="Nenhum estudo pendente de conciliação" />
              </div>
            )}
            {pendentes.data?.map((e) => (
              <button
                key={e.id}
                type="button"
                className="lp"
                aria-current={e.id === selecionado}
                onClick={() => {
                  setSelecionado(e.id);
                  setFeito(null);
                }}
              >
                <span style={{ flex: 1, minWidth: 0 }}>
                  <span className="t" style={{ display: 'block' }}>
                    <span className="num">{e.codigo}</span>, {e.imagens} imagens
                  </span>
                  <span className="s" style={{ display: 'block' }}>
                    {e.accession ?? 'Sem accession'}, recebido {formatarDataHora(e.recebido_em, FUSO_DA_CLINICA)}
                  </span>
                  {e.pendencia !== null && e.pendencia !== undefined && (
                    <span className="t13" style={{ display: 'block', color: 'var(--amber)', fontWeight: 600 }}>
                      {e.pendencia}
                    </span>
                  )}
                </span>
              </button>
            ))}
          </div>
        </section>
        <section className="card pane" aria-label="Conciliação">
          <div className="scroll col" style={{ gap: 14 }}>
            {feito !== null && (
              <Aviso nivel="feito" titulo="Estudo conciliado.">
                O estudo {feito} entrou na fila de laudo do atendimento escolhido.
              </Aviso>
            )}
            {estudo === null ? (
              feito === null && <Vazio titulo="Escolha um estudo da lista" detalhe="A conciliação pede o atendimento e os dois identificadores." />
            ) : (
              <Conciliacao
                key={estudo.id}
                estudo={estudo}
                data={data}
                aoConcluir={() => {
                  setFeito(estudo.codigo);
                  setSelecionado(null);
                  void queryClient.invalidateQueries({ queryKey: CHAVE });
                }}
              />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
