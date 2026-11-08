import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import type { AutorizacaoDetalhe, AutorizacaoResumo, RespostaDoPortal, SituacaoDaAutorizacao } from '@vidacor/api-client';
import { Abas, AreaDeTexto, Aviso, Botao, Campo, CampoDeData, Chip, formatarData, formatarDataHora, GrupoDeOpcoes, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CANAL, chavesDasAutorizacoes, SITUACAO_DA_AUTORIZACAO } from './PedidoDeAutorizacao';

const rota = getRouteApi('/app/autorizacoes');

const FILTROS: { id: string; rotulo: string; situacoes: SituacaoDaAutorizacao[] | null }[] = [
  { id: 'todas', rotulo: 'Todas', situacoes: null },
  { id: 'andamento', rotulo: 'Em andamento', situacoes: ['aguardando_envio', 'aguardando_portal', 'enviada', 'em_analise'] },
  { id: 'pendentes', rotulo: 'Pendentes', situacoes: ['pendente_documento', 'parcial', 'recusada'] },
  { id: 'negadas', rotulo: 'Negadas', situacoes: ['negada'] },
  { id: 'autorizadas', rotulo: 'Autorizadas', situacoes: ['autorizada'] },
];

/**
 * Autorizações (Autorizacao.dc.html): pedidos de senha às operadoras montados a partir do
 * agendamento. Negada, parcial e pendente aparecem primeiro, com o motivo da operadora. Operadora
 * sem webservice: o pedido é feito no portal e o protocolo e a resposta são registrados aqui.
 */
export function TelaAutorizacoes() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const navegar = useNavigate();
  const [filtro, setFiltro] = useState('todas');
  const lista = useQuery({ queryKey: chavesDasAutorizacoes.lista, queryFn: () => api.autorizacoes.listar(), refetchInterval: 30000 });
  const abrir = (id: string) => {
    void navegar({ to: '/autorizacoes', search: { id }, replace: true });
  };
  const linhas = (situacoes: SituacaoDaAutorizacao[] | null): AutorizacaoResumo[] =>
    (lista.data ?? []).filter((a) => situacoes === null || situacoes.includes(a.situacao));

  return (
    <>
      <CabecalhoDaTela titulo="Autorizações" complemento={<span className="t14 mute">Pedidos de senha às operadoras, montados a partir do agendamento</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Pedidos de autorização">
          <div className="scroll">
            <EstadoDaConsulta
              oQue="os pedidos de autorização"
              carregando={lista.isPending}
              erro={lista.error}
              aoTentarDeNovo={() => void lista.refetch()}
              semPermissao={{ titulo: 'Você não vê as autorizações', detalhe: 'Pedidos de autorização são da recepção, do faturamento e da gerência.' }}
            />
            {lista.data !== undefined && (
              <Abas
                rotulo="Filtro das autorizações"
                selecionada={filtro}
                aoSelecionar={setFiltro}
                abas={FILTROS.map((f) => {
                  const itens = linhas(f.situacoes);
                  return {
                    id: f.id,
                    rotulo: f.situacoes === null ? f.rotulo : `${f.rotulo} (${String(itens.length)})`,
                    conteudo: (
                      <div>
                        {itens.length === 0 && <Vazio titulo="Nenhum pedido aqui." detalhe="Mude o filtro." />}
                        {itens.map((a) => (
                          <LinhaDoPaciente
                            key={a.id}
                            nome={a.paciente.nome}
                            nascimento={a.paciente.nascimento}
                            detalhe={`${a.operadora}, ${a.procedimentos.map((p) => p.slice(9)).join(' e ')}`}
                            situacao={SITUACAO_DA_AUTORIZACAO[a.situacao]}
                            {...(a.motivo == null ? {} : { marca: a.motivo })}
                            selecionada={a.id === busca.id}
                            aoAbrir={() => {
                              abrir(a.id);
                            }}
                            contextoDoMenu={a.codigo}
                          />
                        ))}
                      </div>
                    ),
                  };
                })}
              />
            )}
            <p className="t13 mute" style={{ margin: '12px 4px 0' }}>
              Operadoras sem webservice: o pedido é feito no portal e a resposta é registrada aqui, com o mesmo rastro.
            </p>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe da autorização">
          <div className="scroll col" style={{ gap: 12 }}>
            {busca.id === undefined ? <Vazio titulo="Escolha um pedido." detalhe="Negadas e pendentes aparecem primeiro." /> : <Detalhe key={busca.id} id={busca.id} />}
          </div>
        </section>
      </div>
    </>
  );
}

function Detalhe({ id }: { id: string }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const consulta = useQuery({ queryKey: chavesDasAutorizacoes.uma(id), queryFn: () => api.autorizacoes.obter(id) });
  const [erro, setErro] = useState<unknown>(null);
  const [consultando, setConsultando] = useState(false);
  if (consulta.data === undefined) {
    return <EstadoDaConsulta oQue="o pedido" carregando={consulta.isPending} erro={consulta.error} aoTentarDeNovo={() => void consulta.refetch()} />;
  }
  const d = consulta.data;
  const r = d.resumo;
  const s = SITUACAO_DA_AUTORIZACAO[r.situacao];
  const atualizar = async (novo: AutorizacaoDetalhe) => {
    cliente.setQueryData(chavesDasAutorizacoes.uma(id), novo);
    await cliente.invalidateQueries({ queryKey: chavesDasAutorizacoes.lista });
  };
  const aberta = ['enviada', 'em_analise', 'pendente_documento'].includes(r.situacao);
  return (
    <>
      <div className="row wrap" style={{ gap: 10 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{r.paciente.nome}</h2>
          <div className="t14 mute">
            Nascimento <span className="num">{formatarData(r.paciente.nascimento)}</span>, exame em {formatarDataHora(r.inicio_do_exame, FUSO_DA_CLINICA)}
          </div>
        </div>
        <Chip cor={s.cor} texto={s.texto} />
      </div>
      <dl className="grid" style={{ gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: '6px 16px', margin: 0 }}>
        <Dado rotulo="Operadora" valor={r.operadora} />
        <Dado rotulo="Canal" valor={CANAL[r.canal]} />
        <Dado rotulo="Pedido" valor={r.codigo} />
        <Dado rotulo="Protocolo" valor={r.protocolo ?? 'Ainda sem protocolo'} />
        <Dado rotulo="Carteirinha" valor={d.carteirinha} />
        <Dado rotulo="Solicitante" valor={d.solicitante} />
      </dl>
      <section aria-label="Procedimentos do pedido" className="col" style={{ gap: 4 }}>
        {d.itens.map((i) => (
          <div key={i.tuss} className="row t14" style={{ gap: 10 }}>
            <span className="num">{i.tuss}</span>
            <span style={{ flex: 1, minWidth: 0 }}>{i.descricao}</span>
            <span className="num">{i.autorizado == null ? `${String(i.solicitado)} pedido(s)` : `${String(i.autorizado)} de ${String(i.solicitado)} autorizado(s)`}</span>
          </div>
        ))}
      </section>
      {r.motivo != null && ['negada', 'parcial', 'recusada', 'pendente_documento', 'autorizada'].includes(r.situacao) && (
        <Aviso nivel={r.situacao === 'autorizada' ? 'atencao' : 'bloqueante'} titulo={r.situacao === 'negada' ? 'A operadora negou o pedido.' : 'Atenção ao retorno da operadora.'}>
          {r.motivo}
        </Aviso>
      )}
      {d.pendente_no_outbox != null && (
        <Aviso nivel="atencao" titulo="A operadora ainda não deu a resposta final.">
          {`Nova consulta automática em ${formatarDataHora(d.pendente_no_outbox.proxima_em, FUSO_DA_CLINICA)} (tentativa ${String(d.pendente_no_outbox.tentativas)}). ${d.pendente_no_outbox.ultimo_erro ?? ''}`}
        </Aviso>
      )}
      <section aria-label="Andamento" className="col" style={{ gap: 6 }}>
        <h3 className="t15 b" style={{ margin: 0 }}>
          Andamento
        </h3>
        <Passo t="Pedido montado" s="Procedimentos e carteirinha vindos do agendamento" h={r.criado_em} />
        {d.enviada_em != null && <Passo t={r.canal === 'webservice' ? 'Enviado pelo webservice TISS' : 'Pedido feito no portal'} s={r.protocolo == null ? '' : `Protocolo ${r.protocolo}`} h={d.enviada_em} />}
        {[...d.respostas].reverse().map((x) => (
          <Passo
            key={x.sha256}
            t={x.situacao_lida == null ? 'Resposta ilegível, guardada como recebida' : `Resposta: ${SITUACAO_DA_AUTORIZACAO[x.situacao_lida as SituacaoDaAutorizacao].texto}`}
            s={`${x.origem === 'portal' ? 'Registrada do portal' : 'Recebida do webservice'}, SHA-256 ${x.sha256.slice(0, 12)}${x.hash_confere === false ? ', hash do epílogo não confere' : ''}${x.aplicada ? '' : ', sem efeito no pedido'}`}
            h={x.recebida_em}
          />
        ))}
        {d.guia != null && (
          <Passo
            t="Guia preenchida sem digitação"
            s={`Guia ${d.guia.numero}, senha ${d.guia.senha}, válida até ${formatarData(d.guia.validade)}${d.guia.situacao === 'consumida' ? '; reserva confirmada' : '; confirme o agendamento'}`}
            h={d.respondida_em ?? r.criado_em}
          />
        )}
      </section>
      {r.canal === 'webservice' && aberta && (
        <div className="row">
          <Botao
            ocupado={consultando}
            onPress={() => {
              setErro(null);
              setConsultando(true);
              api.autorizacoes
                .consultarAgora(id)
                .then(atualizar, setErro)
                .finally(() => {
                  setConsultando(false);
                });
            }}
          >
            Consultar operadora agora
          </Botao>
        </div>
      )}
      {erro !== null && <ErroDoAto erro={erro} ato="consultar a operadora" />}
      {r.canal === 'portal' && r.situacao === 'aguardando_portal' && <Protocolo id={id} aoConcluir={atualizar} />}
      {r.canal === 'portal' && aberta && <RespostaPortal d={d} aoConcluir={atualizar} />}
    </>
  );
}

function Dado({ rotulo, valor }: { rotulo: string; valor: string }) {
  return (
    <div style={{ minWidth: 0 }}>
      <dt className="t13 mute">{rotulo}</dt>
      <dd className="t14" style={{ margin: 0, overflowWrap: 'anywhere' }}>
        {valor}
      </dd>
    </div>
  );
}

function Passo({ t, s, h }: { t: string; s: string; h: string }) {
  return (
    <div className="col" style={{ gap: 0, paddingLeft: 12, borderLeft: '2px solid var(--line2)' }}>
      <span className="t14 b">{t}</span>
      {s !== '' && <span className="t13">{s}</span>}
      <span className="t13 mute num">{formatarDataHora(h, FUSO_DA_CLINICA)}</span>
    </div>
  );
}

function Protocolo({ id, aoConcluir }: { id: string; aoConcluir: (d: AutorizacaoDetalhe) => Promise<void> }) {
  const { api } = useDependencias();
  const [protocolo, setProtocolo] = useState('');
  const op = useOperacao(() => api.autorizacoes.registrarProtocolo(id, { protocolo: protocolo.trim() }), { aoConcluir });
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Protocolo do portal"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        if (protocolo.trim() !== '') op.enviar(undefined);
      }}
    >
      <h3 className="t15 b" style={{ margin: 0 }}>
        Pedido feito no portal da operadora
      </h3>
      <Campo rotulo="Protocolo do portal" value={protocolo} onChange={setProtocolo} ajuda="Como aparece no portal" />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar o protocolo" />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Registrar protocolo
        </Botao>
      </div>
    </form>
  );
}

const OPCOES_DA_RESPOSTA = [
  { valor: 'autorizada', rotulo: 'Autorizada' },
  { valor: 'parcial', rotulo: 'Em parte' },
  { valor: 'negada', rotulo: 'Negada' },
  { valor: 'em_analise', rotulo: 'Em análise' },
];

function RespostaPortal({ d, aoConcluir }: { d: AutorizacaoDetalhe; aoConcluir: (d: AutorizacaoDetalhe) => Promise<void> }) {
  const { api } = useDependencias();
  const [situacao, setSituacao] = useState<string | null>(null);
  const [guia, setGuia] = useState('');
  const [senha, setSenha] = useState('');
  const [validade, setValidade] = useState<CalendarDate | null>(null);
  const [qtd, setQtd] = useState<Record<string, string>>(Object.fromEntries(d.itens.map((i) => [i.tuss, String(i.solicitado)])));
  const [motivo, setMotivo] = useState('');
  const op = useOperacao(
    (_: undefined, chave: string) => {
      const corpo: RespostaDoPortal = {
        situacao: (situacao ?? 'em_analise') as RespostaDoPortal['situacao'],
        numero_guia_operadora: guia.trim() === '' ? null : guia.trim(),
        senha: senha.trim() === '' ? null : senha.trim(),
        validade: validade?.toString() ?? null,
        procedimentos: ['autorizada', 'parcial'].includes(situacao ?? '') ? d.itens.map((i) => ({ tuss: i.tuss, autorizado: Number(qtd[i.tuss] ?? '0') })) : [],
        motivo: motivo.trim() === '' ? null : motivo.trim(),
      };
      return api.autorizacoes.registrarResposta(d.resumo.id, corpo, { idempotencyKey: chave });
    },
    { aoConcluir },
  );
  const comSenha = situacao === 'autorizada' || situacao === 'parcial';
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Resposta do portal"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        if (situacao !== null) op.enviar(undefined);
      }}
    >
      <h3 className="t15 b" style={{ margin: 0 }}>
        Resposta do portal
      </h3>
      <GrupoDeOpcoes rotulo="O que o portal respondeu" valor={situacao} aoMudar={setSituacao} opcoes={OPCOES_DA_RESPOSTA} />
      {comSenha && (
        <>
          <div className="grid" style={{ gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 10 }}>
            <Campo rotulo="Senha" value={senha} onChange={setSenha} />
            <CampoDeData rotulo="Válida até" valor={validade} aoMudar={setValidade} />
            <Campo rotulo="Número da guia na operadora" value={guia} onChange={setGuia} />
          </div>
          {d.itens.map((i) => (
            <Campo
              key={i.tuss}
              rotulo={`Autorizado de ${i.tuss}`}
              value={qtd[i.tuss] ?? ''}
              inputMode="numeric"
              onChange={(v) => {
                setQtd((a) => ({ ...a, [i.tuss]: v }));
              }}
            />
          ))}
        </>
      )}
      {situacao !== null && situacao !== 'autorizada' && <AreaDeTexto rotulo="Motivo informado pela operadora" value={motivo} onChange={setMotivo} linhas={2} />}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar a resposta" />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando} isDisabled={situacao === null}>
          Registrar resposta
        </Botao>
      </div>
    </form>
  );
}
