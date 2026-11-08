import { ComunicacaoDoPaciente } from './ComunicacaoDoPaciente';
import { ConviteDoPortal } from './ConviteDoPortal';
import { useQuery } from '@tanstack/react-query';
import type { AtendimentoFase1, CorrecaoCadastro, ItemDaAgendaFase1, Reimpressao } from '@vidacor/api-client';
import {
  Aviso,
  Botao,
  CaixaDeMarcar,
  Campo,
  CampoDeData,
  Chip,
  Dialogo,
  EtiquetaPulseira,
  formatarData,
  formatarHora,
  ListaDeDados,
} from '@vidacor/design-system';
import { useState, type ReactNode } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaAtendimento } from '../../consultas';
import { useDependencias } from '../../contexto';
import type { Retomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { aparenciaDoAtendimento } from '../agenda/situacoes';
import { useCheckIn } from '../agenda/useCheckIn';
import { BotaoImprimir, DialogoDeCorrecao, DialogoDeReimpressao, ResultadoDaReimpressao } from './AcoesDaPulseira';

const LAUDO: Record<string, string> = {
  rascunho: 'Em elaboração pelo médico',
  assinado: 'Assinado',
  retificacao_em_rascunho: 'Em retificação',
};

/** Alerta de homônimo em nível "atenção" com os dois nascimentos (FC-CHK-04, R2). */
export function AlertaDeHomonimo({ item }: { item: ItemDaAgendaFase1 }) {
  if (item.homonimos.length === 0) {
    return null;
  }
  return (
    <Aviso nivel="atencao" titulo="Homônimo hoje.">
      {item.homonimos.map((h) => `${h.nome}, nascido em ${formatarData(h.nascimento)}`).join('; ')}
      {item.homonimos.length === 1 ? ' também tem exame hoje.' : ' também têm exame hoje.'} Este paciente nasceu em{' '}
      <b className="num">{formatarData(item.paciente.nascimento)}</b>: confira o nascimento no documento antes de seguir.
    </Aviso>
  );
}

function PulseiraEmitida({ atendimento }: { atendimento: AtendimentoFase1 }) {
  const p = atendimento.paciente;
  const pulseira = atendimento.pulseira ?? null;
  if (pulseira === null) {
    return (
      <Aviso nivel="atencao" titulo="Sem pulseira válida.">
        Nenhuma pulseira deste atendimento está valendo. Reimprima antes de o paciente seguir para a sala.
      </Aviso>
    );
  }
  return (
    <div className="etq-impressao">
      <EtiquetaPulseira
        nome={p.nome}
        nascimento={formatarData(p.nascimento)}
        codigo={pulseira.codigo}
        accession={atendimento.accession}
        situacao={pulseira.situacao}
      />
    </div>
  );
}

function CheckInFeito({ atendimento }: { atendimento: AtendimentoFase1 }) {
  return (
    <Aviso nivel="feito" titulo="Check-in feito.">
      Senha <b className="num">{atendimento.senha}</b>, atendimento <b className="num">{atendimento.codigo}</b>, accession{' '}
      <b className="num">{atendimento.accession}</b>. O paciente aguarda a sala. Imprima a pulseira e coloque no punho.
    </Aviso>
  );
}

function CheckIn({ item, c }: { item: ItemDaAgendaFase1; c: ReturnType<typeof useCheckIn> }) {
  const nasc = formatarData(item.paciente.nascimento);

  return (
    <>
      <p className="t14" style={{ margin: 0, color: 'var(--ink2)', lineHeight: 1.5 }}>
        Pergunte nome completo e data de nascimento e confira com o documento com foto. Não leia os dados para o paciente confirmar.
      </p>
      {c.erroBruto !== null && <ErroDoAto erro={c.erroBruto} ato="fazer o check-in" aoTentarDeNovo={c.enviar} />}
      <Campo rotulo="Nome completo" value={c.nome} onChange={c.setNome} erro={c.errosDosCampos.nome} autoComplete="off" isDisabled={c.enviando} />
      <CampoDeData
        rotulo="Data de nascimento"
        valor={c.nascimento}
        aoMudar={c.setNascimento}
        erro={c.errosDosCampos.nascimento}
        desabilitado={c.enviando}
      />
      <CaixaDeMarcar
        rotulo="Documento com foto conferido"
        detalhe="O rosto do documento confere com o paciente no balcão."
        marcado={c.documento}
        aoMudar={c.setDocumento}
        erro={c.errosDosCampos.documento}
        desabilitado={c.enviando}
      />
      <div className="row">
        <Botao variante="principal" onPress={c.pedirConfirmacao} ocupado={c.enviando}>
          Conferir e fazer check-in
        </Botao>
      </div>
      <Dialogo
        aberto={c.confirmando || c.enviando}
        titulo={`Fazer o check-in de ${item.paciente.nome}?`}
        paciente={{ nome: item.paciente.nome, nascimento: nasc }}
        rotuloConfirmar="Fazer check-in"
        aoConfirmar={c.enviar}
        aoFechar={c.cancelarConfirmacao}
        ocupado={c.enviando}
      >
        <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
          {item.exame.nome}, {formatarHora(item.inicio, FUSO_DA_CLINICA)}, {item.sala.nome}. O servidor confere nome, nascimento e documento
          com o cadastro antes de emitir a senha e a pulseira.
        </p>
      </Dialogo>
    </>
  );
}

interface PropsDoAtendimento {
  item: ItemDaAgendaFase1;
  atendimentoId: string;
  retomada: Retomada | null;
}

function AtendimentoDoPaciente({ item, atendimentoId, retomada }: PropsDoAtendimento) {
  const { api } = useDependencias();
  const consulta = useQuery(consultaAtendimento(api, atendimentoId));
  const [dialogo, setDialogo] = useState<'reimprimir' | 'corrigir' | null>(retomada?.retomar === 'corrigir' ? 'corrigir' : null);
  const [motivoInicial, setMotivoInicial] = useState('');
  const [reimpressao, setReimpressao] = useState<Reimpressao | null>(null);
  const [correcao, setCorrecao] = useState<CorrecaoCadastro | null>(null);

  const estado = (
    <EstadoDaConsulta oQue="o atendimento" carregando={consulta.isPending} erro={consulta.error} aoTentarDeNovo={() => void consulta.refetch()} linhas={2} />
  );
  if (consulta.data === undefined) {
    return estado;
  }
  const a = consulta.data;
  const ap = aparenciaDoAtendimento(a.situacao);
  const codigoParaReimprimir = a.pulseira?.codigo ?? correcao?.etiquetas_invalidadas.at(-1) ?? null;

  let pulseira: ReactNode;
  if (reimpressao !== null) {
    pulseira = <ResultadoDaReimpressao r={reimpressao} atendimento={a} />;
  } else {
    pulseira = <PulseiraEmitida atendimento={a} />;
  }

  return (
    <>
      <div className="row" style={{ gap: 10 }}>
        <h3 className="t15 b">Atendimento</h3>
        <Chip cor={ap.cor} texto={ap.texto} />
      </div>
      <ListaDeDados
        larguraDoRotulo={96}
        itens={[
          { rotulo: 'Senha', valor: <b className="num">{a.senha ?? 'Ainda não emitida'}</b> },
          { rotulo: 'Código', valor: <span className="num">{a.codigo}</span> },
          { rotulo: 'Accession', valor: <span className="num">{a.accession}</span> },
          { rotulo: 'Laudo', valor: a.laudo_situacao === null || a.laudo_situacao === undefined ? 'Ainda não iniciado' : (LAUDO[a.laudo_situacao] ?? a.laudo_situacao) },
        ]}
      />
      {correcao !== null && (
        <Aviso
          nivel="atencao"
          titulo="Cadastro corrigido."
          acoes={
            <Botao
              pequeno
              variante="principal"
              onPress={() => {
                setMotivoInicial('correção de cadastro');
                setDialogo('reimprimir');
              }}
            >
              Reimprimir pulseira
            </Botao>
          }
        >
          Versão {correcao.versao} do cadastro, nascimento {formatarData(correcao.paciente.nascimento)}. As etiquetas{' '}
          {correcao.etiquetas_invalidadas.join(', ')} foram invalidadas. Reimprima a pulseira antes de o paciente seguir para a sala.
        </Aviso>
      )}
      {pulseira}
      <div className="row wrap" style={{ gap: 8 }}>
        <BotaoImprimir />
        <Botao
          pequeno
          isDisabled={codigoParaReimprimir === null}
          onPress={() => {
            setMotivoInicial('');
            setDialogo('reimprimir');
          }}
        >
          Reimprimir pulseira
        </Botao>
        <Botao
          pequeno
          onPress={() => {
            setDialogo('corrigir');
          }}
        >
          Corrigir nascimento
        </Botao>
      </div>
      {dialogo === 'reimprimir' && (
        <DialogoDeReimpressao
          atendimento={a}
          codigo={codigoParaReimprimir ?? ''}
          aberto
          motivoInicial={motivoInicial}
          aoFechar={() => {
            setDialogo(null);
          }}
          aoConcluir={(r) => {
            setReimpressao(r);
            setCorrecao(null);
            setDialogo(null);
          }}
        />
      )}
      {dialogo === 'corrigir' && (
        <DialogoDeCorrecao
          atendimento={a}
          aberto
          retomada={retomada?.retomar === 'corrigir' ? retomada : null}
          agendamentoId={item.id}
          aoFechar={() => {
            setDialogo(null);
          }}
          aoConcluir={(c) => {
            setCorrecao(c);
            setDialogo(null);
          }}
        />
      )}
    </>
  );
}

export interface PropsDoPainel {
  item: ItemDaAgendaFase1;
  retomada: Retomada | null;
}

/** Painel do paciente escolhido: check-in ou, depois dele, senha, pulseira e correções. */
export function PainelDoPaciente({ item, retomada }: PropsDoPainel) {
  // O check-in vive no painel: quando a agenda recarrega com o atendimento, a confirmação continua.
  const c = useCheckIn(item);
  const atendimentoId = c.atendimento?.id ?? item.atendimento_id ?? null;
  return (
    <div className="col" style={{ gap: 14 }}>
      <div>
        <div className="t13 mute">
          {formatarHora(item.inicio, FUSO_DA_CLINICA)}, {item.sala.nome}, {item.exame.nome}
        </div>
        <h2 style={{ fontSize: 20, fontWeight: 650, marginTop: 2 }}>{item.paciente.nome}</h2>
        <div className="t14 num" style={{ color: 'var(--ink2)' }}>
          Nascimento <b>{formatarData(item.paciente.nascimento)}</b>
        </div>
      </div>
      <AlertaDeHomonimo item={item} />
      {c.atendimento !== null && <CheckInFeito atendimento={c.atendimento} />}
      {atendimentoId === null ? (
        <CheckIn item={item} c={c} />
      ) : (
        <AtendimentoDoPaciente key={atendimentoId} item={item} atendimentoId={atendimentoId} retomada={retomada} />
      )}
      <ConviteDoPortal key={item.paciente.id} pacienteId={item.paciente.id} nome={item.paciente.nome} />
      <ComunicacaoDoPaciente key={`c-${item.paciente.id}`} pacienteId={item.paciente.id} />
    </div>
  );
}
