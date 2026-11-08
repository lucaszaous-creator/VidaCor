import { useNavigate } from '@tanstack/react-router';
import type { ItemDaAgenda } from '@vidacor/api-client';
import {
  Aviso,
  Botao,
  BotaoDeIcone,
  Campo,
  CampoDeData,
  Chip,
  Dialogo,
  formatarData,
  formatarHora,
  Gaveta,
  MenuDeContexto,
  useMenuDeContexto,
} from '@vidacor/design-system';
import { useState, type ReactNode } from 'react';
import { FUSO_DA_CLINICA } from './grade';
import { ACOES_DO_AGENDAMENTO, cabecalhoDoMenu, MOTIVO_SEM_ACOES } from './menu';
import { ResumoDoAtendimento } from './ResumoDoAtendimento';
import { aparenciaDoAgendamento } from './situacoes';
import { useCheckIn } from './useCheckIn';

function quando(item: ItemDaAgenda): string {
  return `${formatarHora(item.inicio, FUSO_DA_CLINICA)} às ${formatarHora(item.fim, FUSO_DA_CLINICA)}, ${item.sala.nome}`;
}

/** Botão de mais opções no cabeçalho: o menu de contexto nunca é a única forma (docs/19 § 1). */
function MaisOpcoes({ item }: { item: ItemDaAgenda }) {
  const menu = useMenuDeContexto();
  return (
    <>
      <BotaoDeIcone
        icone="maisOpcoes"
        estilo="mais"
        rotulo={`Mais opções de ${item.paciente.nome}`}
        onPress={(e) => {
          menu.abrirEm(e.target as HTMLElement);
        }}
      />
      <MenuDeContexto
        estado={menu}
        cabecalho={cabecalhoDoMenu(item)}
        acoes={ACOES_DO_AGENDAMENTO}
        motivoSemAcoes={MOTIVO_SEM_ACOES}
        aoEscolher={() => undefined}
      />
    </>
  );
}

interface PropsDaGavetaBase {
  item: ItemDaAgenda;
  rotulo: string;
  rodape?: ReactNode;
  aoFechar: () => void;
  children: ReactNode;
}

function GavetaBase({ item, rotulo, rodape, aoFechar, children }: PropsDaGavetaBase) {
  return (
    <Gaveta
      rotulo={rotulo}
      sobreTitulo={quando(item)}
      titulo={item.paciente.nome}
      subTitulo={`Nascimento ${formatarData(item.paciente.nascimento)}`}
      acoesDoCabecalho={<MaisOpcoes item={item} />}
      rodape={rodape}
      aoFechar={aoFechar}
    >
      {children}
    </Gaveta>
  );
}

function DadosDoAgendamento({ item }: { item: ItemDaAgenda }) {
  const ap = aparenciaDoAgendamento(item.situacao);
  return (
    <>
      <div>
        <Chip cor={ap.cor} texto={ap.texto} />
      </div>
      <dl className="kv" style={{ margin: 0, gridTemplateColumns: '96px minmax(0, 1fr)' }}>
        <dt>Exame</dt>
        <dd>{item.exame.nome}</dd>
        <dt>Sala</dt>
        <dd>{item.sala.nome}</dd>
        <dt>Protocolo</dt>
        <dd className="num">{item.codigo}</dd>
      </dl>
    </>
  );
}

interface PropsDoCheckIn {
  item: ItemDaAgenda;
  aoVoltar: () => void;
  aoFechar: () => void;
}

/**
 * Check-in com dois identificadores (Fluxo-Completo.dc.html, etapa 2). A recepção pergunta e digita
 * o que o paciente diz e o documento mostra; o servidor confere com o cadastro.
 */
function CheckIn({ item, aoVoltar, aoFechar }: PropsDoCheckIn) {
  const c = useCheckIn(item);
  const navegar = useNavigate();
  const nasc = formatarData(item.paciente.nascimento);

  if (c.atendimento !== null) {
    return (
      <GavetaBase
        item={item}
        rotulo="Check-in feito"
        aoFechar={aoFechar}
        rodape={
          <Botao variante="principal" onPress={aoFechar}>
            Voltar à agenda
          </Botao>
        }
      >
        <Aviso nivel="feito" titulo="Check-in feito.">
          Atendimento <b className="num">{c.atendimento.codigo}</b>, accession{' '}
          <b className="num">{c.atendimento.accession}</b>.
        </Aviso>
        <ResumoDoAtendimento atendimentoId={c.atendimento.id} />
      </GavetaBase>
    );
  }

  const erro = c.erro;
  let avisoDeErro: ReactNode = null;
  if (erro !== null) {
    if (erro.tipo === 'sem-conexao') {
      avisoDeErro = (
        <Aviso
          nivel="atencao"
          titulo="Não foi possível falar com o servidor."
          acoes={
            <Botao pequeno onPress={c.enviar} ocupado={c.enviando}>
              Tentar de novo
            </Botao>
          }
        >
          Não dá para saber se o check-in foi registrado. Tente de novo: o reenvio não faz um segundo check-in.
        </Aviso>
      );
    } else if (erro.tipo === 'sem-sessao') {
      avisoDeErro = (
        <Aviso
          nivel="bloqueante"
          titulo="Sua sessão terminou."
          codigoDeSuporte={erro.codigoDeSuporte}
          acoes={
            <Botao pequeno onPress={() => void navegar({ to: '/entrar' })}>
              Entrar de novo
            </Botao>
          }
        >
          O check-in não foi feito. Entre de novo e repita a conferência.
        </Aviso>
      );
    } else {
      avisoDeErro = (
        <Aviso nivel="bloqueante" codigoDeSuporte={erro.codigoDeSuporte}>
          {erro.mensagem}
        </Aviso>
      );
    }
  }

  return (
    <GavetaBase
      item={item}
      rotulo="Check-in do paciente"
      aoFechar={aoFechar}
      rodape={
        <>
          <Botao variante="principal" onPress={c.pedirConfirmacao} ocupado={c.enviando}>
            Conferir e fazer check-in
          </Botao>
          <Botao pequeno onPress={aoVoltar} isDisabled={c.enviando}>
            Voltar
          </Botao>
        </>
      }
    >
      <h3 className="t15 b">Check-in na recepção</h3>
      <p className="t14" style={{ margin: 0, color: 'var(--ink2)', lineHeight: 1.5 }}>
        Pergunte nome completo e data de nascimento e confira com o documento. Não leia os dados para o
        paciente confirmar.
      </p>
      {avisoDeErro}
      <Campo
        rotulo="Nome completo"
        value={c.nome}
        onChange={c.setNome}
        erro={c.errosDosCampos.nome}
        autoComplete="off"
        isDisabled={c.enviando}
      />
      <CampoDeData
        rotulo="Data de nascimento"
        valor={c.nascimento}
        aoMudar={c.setNascimento}
        erro={c.errosDosCampos.nascimento}
        desabilitado={c.enviando}
      />
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
          {item.exame.nome}, {quando(item)}. O servidor confere o nome e o nascimento digitados com o cadastro antes
          de registrar.
        </p>
      </Dialogo>
    </GavetaBase>
  );
}

export interface PropsDaGavetaDoAgendamento {
  item: ItemDaAgenda;
  aoFechar: () => void;
}

export function GavetaDoAgendamento({ item, aoFechar }: PropsDaGavetaDoAgendamento) {
  const [etapa, setEtapa] = useState<'detalhe' | 'checkin'>('detalhe');
  const atendimentoId = item.atendimento_id ?? null;

  if (etapa === 'checkin') {
    return (
      <CheckIn
        item={item}
        aoFechar={aoFechar}
        aoVoltar={() => {
          setEtapa('detalhe');
        }}
      />
    );
  }

  return (
    <GavetaBase
      item={item}
      rotulo="Detalhe do agendamento"
      aoFechar={aoFechar}
      rodape={
        atendimentoId === null ? (
          <Botao
            variante="principal"
            onPress={() => {
              setEtapa('checkin');
            }}
          >
            Fazer check-in
          </Botao>
        ) : undefined
      }
    >
      <DadosDoAgendamento item={item} />
      {atendimentoId !== null && <ResumoDoAtendimento atendimentoId={atendimentoId} />}
    </GavetaBase>
  );
}
