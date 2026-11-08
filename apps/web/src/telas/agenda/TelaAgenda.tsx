import { useQuery } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import type { ItemDaAgendaFase1 } from '@vidacor/api-client';
import {
  Aviso,
  Botao,
  BotaoDeIcone,
  Carregando,
  ErroTela,
  formatarData,
  formatarDiaPorExtenso,
  SemConexao,
  SemPermissao,
  Vazio,
} from '@vidacor/design-system';
import { useId, useState } from 'react';
import { consultaAgenda, consultaSalas, consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { erroParaTela } from '../../erros';
import { useEventosAoVivo } from '../../eventos/EventosAoVivo';
import { ConfirmarPorWhatsapp } from './ConfirmarPorWhatsapp';
import { GavetaDoAgendamento } from './GavetaDoAgendamento';
import { hojeNaClinica, montarGrade, somarDias } from './grade';
import { GradeDaAgenda } from './GradeDaAgenda';

const rota = getRouteApi('/app/agenda');

function AvisoDeEstudo({ itens }: { itens: ItemDaAgendaFase1[] }) {
  const { recebidos, foraDoAr } = useEventosAoVivo();
  const ultimo = recebidos[0];
  return (
    <>
      {foraDoAr && (
        <Aviso nivel="atencao" titulo="Avisos em tempo real fora do ar.">
          Tentando reconectar. A agenda continua atualizando a cada minuto.
        </Aviso>
      )}
      {ultimo !== undefined &&
        (() => {
          const item = itens.find((i) => i.atendimento_id === ultimo.atendimento_id);
          const imagens = `${String(ultimo.imagens)} ${ultimo.imagens === 1 ? 'imagem' : 'imagens'}`;
          return (
            <Aviso nivel="informativo" titulo="Estudo recebido.">
              {item === undefined
                ? `Accession ${ultimo.accession}, ${imagens}.`
                : `${item.paciente.nome}, nascimento ${formatarData(item.paciente.nascimento)}. Accession ${ultimo.accession}, ${imagens}.`}
            </Aviso>
          );
        })()}
    </>
  );
}

/** Agenda do dia por sala (Agenda.dc.html), com check-in pela gaveta. */
export function TelaAgenda() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const navegar = rota.useNavigate();
  const idSala = useId();
  const data = busca.data ?? hojeNaClinica();
  const [selecionado, setSelecionado] = useState<string | null>(null);

  const salas = useQuery(consultaSalas(api));
  const sessao = useQuery(consultaSessao(api));
  const podeConfirmar = sessao.data?.usuario.perfis.some((p) => p === 'recepcao' || p === 'gerente') ?? false;
  const agenda = useQuery(consultaAgenda(api, data, busca.sala));

  function irPara(novaData: string): void {
    setSelecionado(null);
    void navegar({ search: (s) => ({ ...s, data: novaData }) });
  }

  const itemSelecionado = agenda.data?.itens.find((i) => i.id === selecionado) ?? null;

  let conteudo;
  const erro = agenda.error ?? salas.error;
  const dadosDaAgenda = agenda.data;
  const dadosDasSalas = salas.data;
  if (erro !== null) {
    const e = erroParaTela(erro);
    const tentar = (
      <Botao
        pequeno
        onPress={() => {
          void agenda.refetch();
          void salas.refetch();
        }}
      >
        Tentar de novo
      </Botao>
    );
    conteudo = (
      <div style={{ padding: 22 }}>
        {e.tipo === 'sem-permissao' ? (
          <SemPermissao titulo="Você não tem acesso à agenda" detalhe="Peça à coordenação o perfil com acesso à agenda." />
        ) : e.tipo === 'sem-conexao' ? (
          <SemConexao acao={tentar}>A agenda não carregou. Nada foi alterado.</SemConexao>
        ) : (
          <ErroTela mensagem={e.mensagem} codigoDeSuporte={e.codigoDeSuporte} acao={tentar} />
        )}
      </div>
    );
  } else if (dadosDaAgenda === undefined || dadosDasSalas === undefined) {
    conteudo = (
      <div style={{ padding: 22 }}>
        <Carregando rotulo="Carregando a agenda" linhas={5} />
      </div>
    );
  } else if (dadosDaAgenda.itens.length === 0) {
    conteudo = (
      <div style={{ padding: 22 }}>
        <Vazio titulo="Nenhum agendamento neste dia" detalhe={busca.sala === undefined ? undefined : 'Nesta sala.'} />
      </div>
    );
  } else {
    const grade = montarGrade(dadosDaAgenda.itens, dadosDasSalas, busca.sala);
    conteudo = (
      <div className="scroll">
        {grade.semHorario.length > 0 && (
          <div style={{ padding: 12 }}>
            <Aviso nivel="atencao" titulo="Agendamentos com horário ilegível.">
              {grade.semHorario.map((i) => i.paciente.nome).join(', ')}. Avise a TI.
            </Aviso>
          </div>
        )}
        <GradeDaAgenda
          grade={grade}
          selecionado={selecionado}
          aoAbrir={(item) => {
            setSelecionado(item.id);
          }}
        />
      </div>
    );
  }

  return (
    <>
      <header className="row wrap" style={{ gap: 10 }}>
        <BotaoDeIcone icone="anterior" rotulo="Dia anterior" onPress={() => { irPara(somarDias(data, -1)); }} />
        <Botao pequeno onPress={() => { irPara(hojeNaClinica()); }}>
          Hoje
        </Botao>
        <BotaoDeIcone icone="proximo" rotulo="Próximo dia" onPress={() => { irPara(somarDias(data, 1)); }} />
        <h1 style={{ margin: '0 0 0 6px', fontSize: 24, fontWeight: 650, letterSpacing: '-.02em' }}>
          {formatarDiaPorExtenso(data)}
        </h1>
        <div className="grow" />
        {podeConfirmar && <ConfirmarPorWhatsapp data={data} />}
        <label className="sr" htmlFor={idSala}>
          Salas
        </label>
        <select
          id={idSala}
          className="in ag-sala"
          value={busca.sala ?? ''}
          onChange={(ev) => {
            setSelecionado(null);
            const sala = ev.target.value;
            void navegar({ search: (s) => ({ ...s, sala: sala === '' ? undefined : sala }) });
          }}
        >
          <option value="">Todas as salas</option>
          {(salas.data ?? []).map((s) => (
            <option key={s.id} value={s.id}>
              {s.nome}
            </option>
          ))}
        </select>
      </header>

      <div className="row wrap t13 mute" style={{ gap: 16, padding: '0 4px' }}>
        <span>
          {agenda.data === undefined ? (
            'Agendados: carregando'
          ) : (
            <>
              <b style={{ color: 'var(--ink)' }}>{agenda.data.itens.length}</b> agendados
            </>
          )}
        </span>
        <span className="grow" />
        <span className="ag-legenda">
          <i className="dot" style={{ background: '#5B45B5' }} aria-hidden="true" />
          Em atendimento
        </span>
        <span className="ag-legenda">
          <i className="dot" style={{ background: '#CBD2DC' }} aria-hidden="true" />
          Concluído
        </span>
      </div>

      <AvisoDeEstudo itens={agenda.data?.itens ?? []} />

      <div className="fill" style={{ position: 'relative', display: 'flex', flexDirection: 'column' }}>
        <section className="card pane" aria-label="Agenda do dia por sala" style={{ padding: 0, flex: '1 1 auto' }}>
          {conteudo}
        </section>
        {itemSelecionado !== null && (
          <GavetaDoAgendamento
            key={itemSelecionado.id}
            item={itemSelecionado}
            aoFechar={() => {
              setSelecionado(null);
            }}
          />
        )}
      </div>
    </>
  );
}
