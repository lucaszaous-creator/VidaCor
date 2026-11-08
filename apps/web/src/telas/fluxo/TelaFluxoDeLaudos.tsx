import { useQuery } from "@tanstack/react-query";
import type { CartaoDoFluxo, EtapaDoLaudo } from "@vidacor/api-client";
import {
  Abas,
  Chip,
  Escolha,
  formatarData,
  formatarHora,
  Vazio,
} from "@vidacor/design-system";
import { useState } from "react";
import { CabecalhoDaTela, EstadoDaConsulta } from "../../componentes/Bloco";
import { useDependencias } from "../../contexto";
import { FUSO_DA_CLINICA } from "../agenda/grade";

const NOME_DA_ETAPA: Record<EtapaDoLaudo, string> = {
  a_laudar: "A laudar",
  em_retificacao: "Em retificação",
  aguardando_conversa: "Aguardando conversa",
  assinado: "Assinado",
  entregue: "Entregue",
};

function Cartao({ c }: { c: CartaoDoFluxo }) {
  return (
    <li
      style={{
        padding: 8,
        borderRadius: 10,
        border: "1px solid var(--line)",
        background: "var(--card, #fff)",
        minWidth: 0,
      }}
    >
      <b className="t14" style={{ display: "block", overflowWrap: "anywhere" }}>
        {c.paciente_nome}, {formatarData(c.paciente_nascimento)}
      </b>
      <span className="t13 mute" style={{ display: "block" }}>
        {c.exame}, recebido {formatarHora(c.recebido_em, FUSO_DA_CLINICA)}
        {c.medico === null || c.medico === undefined ? "" : `, ${c.medico}`}
      </span>
      <div className="row" style={{ gap: 6, marginTop: 4 }}>
        {c.atrasado && <Chip cor="rosa" texto="Fora do prazo" />}
        {c.versao_vigente !== null &&
          c.versao_vigente !== undefined &&
          c.versao_vigente > 1 && (
            <Chip cor="violeta" texto={`v${String(c.versao_vigente)}`} />
          )}
      </div>
    </li>
  );
}

/**
 * Fluxo de laudos (Fluxo-Laudos.dc.html) para a coordenação: quadro por etapa com prazo, produção por
 * médico e retificações. Só metadados; o texto do laudo fica na tela do laudo, com vínculo.
 */
export function TelaFluxoDeLaudos() {
  const { api } = useDependencias();
  const f = useQuery({
    queryKey: ["fluxo-laudos"],
    queryFn: () => api.fluxoDeLaudos(),
    refetchInterval: 60_000,
  });
  const [medico, setMedico] = useState("");
  const [modalidade, setModalidade] = useState("");
  if (f.data === undefined) {
    return (
      <>
        <CabecalhoDaTela titulo="Fluxo de laudos" />
        <EstadoDaConsulta
          oQue="o fluxo de laudos"
          carregando={f.isPending}
          erro={f.error}
          aoTentarDeNovo={() => void f.refetch()}
        />
      </>
    );
  }
  const d = f.data;
  const medicos = [
    ...new Set(d.cartoes.map((c) => c.medico ?? "").filter((m) => m !== "")),
  ].sort();
  const modalidades = [...new Set(d.cartoes.map((c) => c.modalidade))].sort();
  const filtrados = d.cartoes.filter(
    (c) =>
      (medico === "" || c.medico === medico) &&
      (modalidade === "" || c.modalidade === modalidade),
  );
  return (
    <>
      <CabecalhoDaTela
        titulo="Fluxo de laudos"
        complemento={
          <span className="t14 mute">
            Prazo de {d.prazo_horas} h desde as imagens.{" "}
            {d.no_prazo_percentual === null ||
            d.no_prazo_percentual === undefined
              ? ""
              : `${String(d.no_prazo_percentual)}% no prazo hoje, `}
            {d.vencem_em_2h} vencem em 2 h, {d.atrasados} fora do prazo.
          </span>
        }
      />
      <div className="row wrap" style={{ gap: 10 }}>
        <div style={{ width: 240 }}>
          <Escolha
            rotulo="Médico"
            valor={medico}
            aoMudar={setMedico}
            vazio="Todos os médicos"
            opcoes={medicos.map((m) => ({ valor: m, rotulo: m }))}
          />
        </div>
        <div style={{ width: 200 }}>
          <Escolha
            rotulo="Modalidade"
            valor={modalidade}
            aoMudar={setModalidade}
            vazio="Todas as modalidades"
            opcoes={modalidades.map((m) => ({ valor: m, rotulo: m }))}
          />
        </div>
      </div>
      <section
        className="card pane fill"
        aria-label="Fluxo"
        style={{ marginTop: 12 }}
      >
        <Abas
          rotulo="Fluxo"
          abas={[
            {
              id: "quadro",
              rotulo: "Quadro por etapa",
              conteudo: (
                <div
                  role="region"
                  aria-label="Quadro por etapa"
                  style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(5, minmax(0, 1fr))",
                    gap: 10,
                    height: "calc(100vh - 300px)",
                    minHeight: 260,
                  }}
                >
                  {d.etapas.map((e) => {
                    const cartoes = filtrados.filter(
                      (c) => c.etapa === e.etapa,
                    );
                    return (
                      <div
                        key={e.etapa}
                        className="col"
                        style={{ gap: 6, minHeight: 0, minWidth: 0 }}
                      >
                        <h2 className="t14 b" style={{ margin: 0 }}>
                          {NOME_DA_ETAPA[e.etapa]}{" "}
                          <span className="num mute">{cartoes.length}</span>
                        </h2>
                        <ul
                          className="col"
                          style={{
                            gap: 6,
                            listStyle: "none",
                            margin: 0,
                            padding: 0,
                            overflowY: "auto",
                            overflowX: "hidden",
                            minHeight: 0,
                            flex: 1,
                          }}
                          aria-label={NOME_DA_ETAPA[e.etapa]}
                        >
                          {cartoes.length === 0 ? (
                            <li className="t13 mute">Nada neste recorte</li>
                          ) : (
                            cartoes.map((c) => (
                              <Cartao key={c.laudo_id} c={c} />
                            ))
                          )}
                        </ul>
                      </div>
                    );
                  })}
                </div>
              ),
            },
            {
              id: "producao",
              rotulo: "Produção",
              conteudo:
                d.producao.length === 0 ? (
                  <Vazio titulo="Sem laudos no período." />
                ) : (
                  <ul
                    className="col"
                    style={{ gap: 0, listStyle: "none", margin: 0, padding: 0 }}
                  >
                    {d.producao.map((p) => (
                      <li
                        key={p.medico}
                        style={{
                          padding: "8px 0",
                          borderBottom: "1px solid var(--line)",
                        }}
                      >
                        <b style={{ display: "block" }}>{p.medico}</b>
                        <span className="t13 mute">
                          {p.assinados_hoje} assinados hoje
                          {p.tempo_medio_horas === null ||
                          p.tempo_medio_horas === undefined
                            ? ""
                            : `, ${String(p.tempo_medio_horas).replace(".", ",")} h em média`}
                          , {p.na_fila} na fila
                        </span>
                      </li>
                    ))}
                  </ul>
                ),
            },
            {
              id: "retificacoes",
              rotulo: `Retificações (${String(d.retificacoes.length)})`,
              conteudo:
                d.retificacoes.length === 0 ? (
                  <Vazio titulo="Nenhuma retificação nos últimos 7 dias." />
                ) : (
                  <ul
                    className="col"
                    style={{ gap: 0, listStyle: "none", margin: 0, padding: 0 }}
                  >
                    {d.retificacoes.map((r) => (
                      <li
                        key={`${r.laudo_id}-${String(r.versao)}`}
                        style={{
                          padding: "8px 0",
                          borderBottom: "1px solid var(--line)",
                        }}
                      >
                        <b style={{ display: "block" }}>
                          {r.paciente_nome}, {r.exame}
                        </b>
                        <span className="t13 mute">
                          v{r.versao}
                          {r.medico === null || r.medico === undefined
                            ? ""
                            : `, ${r.medico}`}
                          . {r.motivo ?? ""} A versão anterior segue guardada.
                        </span>
                      </li>
                    ))}
                  </ul>
                ),
            },
          ]}
        />
      </section>
    </>
  );
}
