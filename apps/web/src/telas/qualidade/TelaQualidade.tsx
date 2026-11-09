import { useQuery, useQueryClient } from "@tanstack/react-query";
import type {
  AndamentoNovo,
  EventoDeSeguranca,
  NaoConformidade,
  SituacaoDaNc,
} from "@vidacor/api-client";
import {
  Abas,
  AreaDeTexto,
  Aviso,
  Botao,
  Chip,
  Escolha,
  formatarData,
  formatarDataHora,
  ListaDeDados,
  SimOuNao,
  Vazio,
} from "@vidacor/design-system";
import { useState } from "react";
import { CabecalhoDaTela, EstadoDaConsulta } from "../../componentes/Bloco";
import { ErroDoAto } from "../../componentes/ErroDoAto";
import { consultaSessao } from "../../consultas";
import { useDependencias } from "../../contexto";
import { useOperacao } from "../../operacao";
import { FUSO_DA_CLINICA } from "../agenda/grade";
import {
  AbrirNaoConformidade,
  CLASSIFICACOES,
  GRAVIDADES_EVENTO,
  NotificarEvento,
  TIPOS_DE_EVENTO,
} from "./FormulariosDaQualidade";

const SITUACOES: { valor: SituacaoDaNc; rotulo: string }[] = [
  { valor: "aberta", rotulo: "Aberta" },
  { valor: "em_acao", rotulo: "Em ação" },
  { valor: "verificacao", rotulo: "Verificação da eficácia" },
  { valor: "encerrada", rotulo: "Encerrada" },
];
const nome = (lista: { valor: string; rotulo: string }[], v: string) =>
  lista.find((x) => x.valor === v)?.rotulo ?? v;

function DetalheDaNc({ nc }: { nc: NaoConformidade }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [texto, setTexto] = useState("");
  const [situacao, setSituacao] = useState<string>(
    nc.situacao === "aberta" ? "em_acao" : nc.situacao,
  );
  const [eficaz, setEficaz] = useState<boolean | null>(null);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (a: AndamentoNovo, chave: string) =>
      api.qualidade.andamento(nc.id, a, { idempotencyKey: chave }),
    {
      aoConcluir: () => cliente.invalidateQueries({ queryKey: ["qualidade"] }),
    },
  );
  const encerrando = situacao === "encerrada";
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {nc.codigo}
        </h2>
        <Chip
          cor={
            nc.situacao === "encerrada"
              ? "verde"
              : nc.vencida
                ? "rosa"
                : "ambar"
          }
          texto={nc.vencida ? "Prazo vencido" : nome(SITUACOES, nc.situacao)}
        />
      </div>
      <p className="t14" style={{ margin: 0, overflowWrap: "anywhere" }}>
        {nc.descricao}
      </p>
      <ListaDeDados
        larguraDoRotulo={120}
        itens={[
          {
            rotulo: "Classificação",
            valor: `${nome(CLASSIFICACOES, nc.classificacao)}, gravidade ${nc.gravidade}`,
          },
          {
            rotulo: "Aberta",
            valor: `${formatarDataHora(nc.aberta_em, FUSO_DA_CLINICA)} por ${nc.aberta_por}`,
          },
          { rotulo: "Responsável", valor: nc.responsavel ?? "A definir" },
          {
            rotulo: "Prazo",
            valor:
              nc.prazo_em === null || nc.prazo_em === undefined
                ? "A definir"
                : formatarData(nc.prazo_em),
          },
        ]}
      />
      {nc.andamentos.length > 0 && (
        <ul
          className="col"
          style={{ gap: 6, listStyle: "none", margin: 0, padding: 0 }}
          aria-label="Andamentos"
        >
          {nc.andamentos.map((a, k) => (
            <li
              key={k}
              className="t13"
              style={{ borderLeft: "2px solid var(--line)", paddingLeft: 8 }}
            >
              <b>{nome(SITUACOES, a.situacao_nova)}</b>, {a.por},{" "}
              {formatarDataHora(a.em, FUSO_DA_CLINICA)}: {a.texto}
            </li>
          ))}
        </ul>
      )}
      {op.resultado !== null && (
        <Aviso nivel="feito" titulo="Andamento registrado.">
          A versão anterior segue guardada no histórico.
        </Aviso>
      )}
      {nc.situacao !== "encerrada" && op.resultado === null && (
        <>
          <Escolha
            rotulo="Nova situação"
            valor={situacao}
            aoMudar={setSituacao}
            opcoes={SITUACOES.filter((s) => s.valor !== "aberta")}
          />
          <AreaDeTexto
            rotulo="Andamento"
            value={texto}
            onChange={setTexto}
            linhas={3}
            erro={
              tentou && texto.trim().length < 5 ? "Escreva o andamento." : null
            }
          />
          {encerrando && (
            <SimOuNao
              pergunta="A ação foi eficaz?"
              valor={eficaz}
              aoMudar={setEficaz}
              erro={
                tentou && eficaz === null
                  ? "Diga se a ação foi eficaz para encerrar."
                  : null
              }
            />
          )}
          {op.erro !== null && (
            <ErroDoAto erro={op.erro} ato="registrar o andamento" />
          )}
          <div className="row">
            <Botao
              variante="principal"
              ocupado={op.enviando}
              onPress={() => {
                setTentou(true);
                if (
                  texto.trim().length >= 5 &&
                  (!encerrando || eficaz !== null)
                )
                  op.enviar({
                    texto: texto.trim(),
                    situacao: situacao as SituacaoDaNc,
                    eficaz: encerrando ? eficaz : null,
                    versao: nc.versao,
                  });
              }}
            >
              Registrar andamento
            </Botao>
          </div>
        </>
      )}
    </div>
  );
}

function DetalheDoEvento({ e }: { e: EventoDeSeguranca }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const proxima =
    e.situacao === "recebido"
      ? "em_analise"
      : e.situacao === "em_analise"
        ? "concluido"
        : null;
  const op = useOperacao(
    (_: undefined, chave: string) =>
      api.qualidade.situacaoDoEvento(e.id, proxima ?? "em_analise", {
        idempotencyKey: chave,
      }),
    {
      aoConcluir: () => cliente.invalidateQueries({ queryKey: ["qualidade"] }),
    },
  );
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {e.codigo}, {nome(TIPOS_DE_EVENTO, e.tipo)}
        </h2>
        <Chip
          cor={e.situacao === "concluido" ? "verde" : "ambar"}
          texto={
            e.situacao === "recebido"
              ? "Recebido"
              : e.situacao === "em_analise"
                ? "Em análise"
                : "Concluído"
          }
        />
      </div>
      {e.notificar_anvisa_ate !== null &&
        e.notificar_anvisa_ate !== undefined &&
        e.situacao !== "concluido" && (
          <Aviso
            nivel="bloqueante"
            titulo={`Notificar a Anvisa até ${formatarDataHora(e.notificar_anvisa_ate, FUSO_DA_CLINICA)}.`}
          >
            Dano grave ou óbito: 72 h para a notificação no Notivisa (RDC
            36/2013).
          </Aviso>
        )}
      <ListaDeDados
        larguraDoRotulo={130}
        itens={[
          {
            rotulo: "Gravidade",
            valor: e.quase_erro
              ? "Quase-erro, sem dano"
              : nome(GRAVIDADES_EVENTO, e.gravidade),
          },
          { rotulo: "Paciente", valor: e.paciente ?? "Não informado" },
          {
            rotulo: "Notificado",
            valor: `${formatarDataHora(e.notificado_em, FUSO_DA_CLINICA)}${e.notificado_por === null || e.notificado_por === undefined ? ", anônimo" : ` por ${e.notificado_por}`}`,
          },
        ]}
      />
      <p
        className="t14"
        style={{ margin: 0, whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}
      >
        {e.descricao}
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="mudar a situação" />}
      {proxima !== null && op.resultado === null && (
        <div className="row">
          <Botao
            ocupado={op.enviando}
            onPress={() => {
              op.enviar(undefined);
            }}
          >
            {proxima === "em_analise" ? "Iniciar análise" : "Concluir análise"}
          </Botao>
        </div>
      )}
      {op.resultado !== null && (
        <Aviso nivel="feito" titulo="Situação registrada no lastro." />
      )}
    </div>
  );
}

function Painel() {
  const { api } = useDependencias();
  const p = useQuery({
    queryKey: ["qualidade"],
    queryFn: () => api.qualidade.painel(),
  });
  const [sel, setSel] = useState<{
    tipo: "nc" | "evento" | "abrir" | "notificar";
    id?: string;
  } | null>(null);
  const nc =
    sel?.tipo === "nc"
      ? p.data?.nao_conformidades.find((n) => n.id === sel.id)
      : undefined;
  const ev =
    sel?.tipo === "evento"
      ? p.data?.eventos.find((e) => e.id === sel.id)
      : undefined;
  const botao = (
    rotulo: string,
    ativo: boolean,
    aoClicar: () => void,
    linha2: string,
    chip: { cor: "verde" | "ambar" | "rosa"; texto: string },
  ) => (
    <button
      type="button"
      aria-current={ativo ? "true" : undefined}
      style={{
        width: "100%",
        textAlign: "left",
        display: "flex",
        gap: 10,
        alignItems: "center",
        padding: "10px 8px",
        border: 0,
        background: ativo ? "var(--line)" : "transparent",
        borderBottom: "1px solid var(--line)",
        cursor: "pointer",
        font: "inherit",
        color: "inherit",
      }}
      onClick={aoClicar}
    >
      <span style={{ flex: 1, minWidth: 0 }}>
        <b style={{ display: "block", overflowWrap: "anywhere" }}>{rotulo}</b>
        <span className="t13 mute">{linha2}</span>
      </span>
      <Chip cor={chip.cor} texto={chip.texto} />
    </button>
  );
  return (
    <>
      <CabecalhoDaTela
        titulo="Qualidade"
        complemento={
          p.data === undefined ? undefined : (
            <span className="t14 mute">
              {p.data.nc_abertas} não conformidades abertas (
              {p.data.nc_vencidas} vencidas), {p.data.eventos_no_mes} eventos no
              mês ({p.data.quase_erros_no_mes} quase-erros,{" "}
              {p.data.graves_no_mes} graves)
            </span>
          )
        }
      >
        <Botao
          pequeno
          onPress={() => {
            setSel({ tipo: "abrir" });
          }}
        >
          Abrir não conformidade
        </Botao>
        <Botao
          pequeno
          variante="principal"
          onPress={() => {
            setSel({ tipo: "notificar" });
          }}
        >
          Notificar evento
        </Botao>
      </CabecalhoDaTela>
      {p.data !== undefined && p.data.anvisa_pendentes > 0 && (
        <Aviso
          nivel="bloqueante"
          titulo={`${String(p.data.anvisa_pendentes)} evento(s) grave(s) aguardando notificação à Anvisa.`}
        >
          O prazo é de 72 h a partir da notificação interna.
        </Aviso>
      )}
      <div
        className="fill"
        style={{
          display: "grid",
          gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1.2fr)",
          gridTemplateRows: "minmax(0, 1fr)",
          gap: 16,
        }}
      >
        <section className="card pane" aria-label="Registros da qualidade">
          <div className="scroll">
            <EstadoDaConsulta
              oQue="a qualidade"
              carregando={p.isPending}
              erro={p.error}
              aoTentarDeNovo={() => void p.refetch()}
            />
            {p.data !== undefined && (
              <Abas
                rotulo="Registros da qualidade"
                abas={[
                  {
                    id: "nc",
                    rotulo: `Não conformidades (${String(p.data.nc_abertas)})`,
                    conteudo:
                      p.data.nao_conformidades.length === 0 ? (
                        <Vazio titulo="Nenhuma não conformidade." />
                      ) : (
                        <ul
                          style={{ listStyle: "none", margin: 0, padding: 0 }}
                        >
                          {p.data.nao_conformidades.map((n) => (
                            <li key={n.id}>
                              {botao(
                                n.descricao.slice(0, 80),
                                sel?.id === n.id,
                                () => {
                                  setSel({ tipo: "nc", id: n.id });
                                },
                                `${n.codigo}, ${nome(CLASSIFICACOES, n.classificacao)}`,
                                {
                                  cor:
                                    n.situacao === "encerrada"
                                      ? "verde"
                                      : n.vencida
                                        ? "rosa"
                                        : "ambar",
                                  texto: n.vencida
                                    ? "Vencida"
                                    : nome(SITUACOES, n.situacao),
                                },
                              )}
                            </li>
                          ))}
                        </ul>
                      ),
                  },
                  {
                    id: "eventos",
                    rotulo: `Eventos (${String(p.data.eventos.length)})`,
                    conteudo:
                      p.data.eventos.length === 0 ? (
                        <Vazio titulo="Nenhum evento notificado." />
                      ) : (
                        <ul
                          style={{ listStyle: "none", margin: 0, padding: 0 }}
                        >
                          {p.data.eventos.map((e) => (
                            <li key={e.id}>
                              {botao(
                                nome(TIPOS_DE_EVENTO, e.tipo),
                                sel?.id === e.id,
                                () => {
                                  setSel({ tipo: "evento", id: e.id });
                                },
                                `${e.codigo}, ${e.quase_erro ? "quase-erro" : nome(GRAVIDADES_EVENTO, e.gravidade)}`,
                                {
                                  cor:
                                    e.situacao === "concluido"
                                      ? "verde"
                                      : e.notificar_anvisa_ate !== null &&
                                          e.notificar_anvisa_ate !== undefined
                                        ? "rosa"
                                        : "ambar",
                                  texto:
                                    e.situacao === "recebido"
                                      ? "Recebido"
                                      : e.situacao === "em_analise"
                                        ? "Em análise"
                                        : "Concluído",
                                },
                              )}
                            </li>
                          ))}
                        </ul>
                      ),
                  },
                ]}
              />
            )}
            <p className="t13 mute" style={{ margin: "8px 0 0" }}>
              Eventos visíveis só ao Núcleo de Segurança do Paciente. Nada é
              apagado.
            </p>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe da qualidade">
          <div className="scroll">
            {sel?.tipo === "abrir" ? (
              <AbrirNaoConformidade />
            ) : sel?.tipo === "notificar" ? (
              <NotificarEvento />
            ) : nc !== undefined ? (
              <DetalheDaNc key={nc.id} nc={nc} />
            ) : ev !== undefined ? (
              <DetalheDoEvento key={ev.id} e={ev} />
            ) : (
              <Vazio
                titulo="Escolha um registro."
                detalhe="Ou abra uma não conformidade ou notifique um evento."
              />
            )}
          </div>
        </section>
      </div>
    </>
  );
}

/** Qualidade (Qualidade.dc.html): a gerência (NSP) vê e trata; a equipe abre e notifica. */
export function TelaQualidade() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  if (sessao.data === undefined) return null;
  if (sessao.data.usuario.perfis.includes("gerente")) return <Painel />;
  return (
    <>
      <CabecalhoDaTela
        titulo="Qualidade e segurança do paciente"
        complemento={
          <span className="t14 mute">
            O que você registrar vai para a qualidade e para o Núcleo de
            Segurança do Paciente.
          </span>
        }
      />
      <div
        className="fill"
        style={{
          display: "grid",
          gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)",
          gridTemplateRows: "minmax(0, 1fr)",
          gap: 16,
        }}
      >
        <section className="card pane" aria-label="Notificar evento">
          <div className="scroll">
            <NotificarEvento />
          </div>
        </section>
        <section className="card pane" aria-label="Não conformidade">
          <div className="scroll">
            <AbrirNaoConformidade />
          </div>
        </section>
      </div>
    </>
  );
}
