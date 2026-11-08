import { useQuery } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";
import type { ResultadoDaBusca } from "@vidacor/api-client";
import { BotaoDeIcone, Busca, Janela } from "@vidacor/design-system";
import { useEffect, useState } from "react";
import { ErroDoAto } from "../componentes/ErroDoAto";
import { useDependencias } from "../contexto";
import { secoesDoMenu } from "./menu";

const NOME_DO_TIPO: Record<ResultadoDaBusca["tipo"], string> = {
  paciente: "Paciente",
  agendamento: "Agendamento",
  atendimento: "Atendimento",
  etiqueta: "Etiqueta",
  laudo: "Laudo",
};

function useAtrasado(valor: string, ms: number): string {
  const [v, setV] = useState(valor);
  useEffect(() => {
    const t = setTimeout(() => {
      setV(valor);
    }, ms);
    return () => {
      clearTimeout(t);
    };
  }, [valor, ms]);
  return v;
}

/**
 * Busca global (Busca-Global.dc.html): Ctrl+K ou o botão de lupa. Pacientes por nome, nascimento ou
 * celular e objetos por código vêm do servidor, só o que o perfil pode ver; começando com ">" lista as
 * telas do menu do perfil.
 */
export function BuscaGlobal({ perfis }: { perfis: readonly string[] }) {
  const { api } = useDependencias();
  const router = useRouter();
  const [aberto, setAberto] = useState(false);
  const [q, setQ] = useState("");
  const termo = useAtrasado(q.trim(), 300);
  const acoes = termo.startsWith(">");
  const consulta = useQuery({
    queryKey: ["busca", termo],
    queryFn: () => api.buscar(termo),
    enabled: aberto && !acoes && termo.length >= 3,
    retry: false,
  });
  useEffect(() => {
    function aoTeclar(ev: KeyboardEvent): void {
      if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "k") {
        ev.preventDefault();
        setAberto(true);
      }
    }
    window.addEventListener("keydown", aoTeclar);
    return () => {
      window.removeEventListener("keydown", aoTeclar);
    };
  }, []);
  function abrir(destino: string): void {
    setAberto(false);
    setQ("");
    void router.navigate({ href: destino });
  }
  const telas = acoes
    ? secoesDoMenu(perfis)
        .flatMap((s) => s.itens)
        .filter((i) =>
          i.rotulo.toLowerCase().includes(termo.slice(1).trim().toLowerCase()),
        )
    : [];
  return (
    <>
      <BotaoDeIcone
        icone="busca"
        rotulo="Buscar em tudo (Ctrl K)"
        onPress={() => {
          setAberto(true);
        }}
      />
      <Janela
        aberto={aberto}
        titulo="Buscar em tudo"
        tituloOculto
        aoFechar={() => {
          setAberto(false);
        }}
      >
        <div className="col" style={{ gap: 10 }}>
          <Busca
            rotulo="Buscar pacientes, códigos ou telas"
            valor={q}
            aoMudar={setQ}
            placeholder="Nome, nascimento (dd/mm/aaaa), celular ou código; > para telas"
            atalho="Esc"
          />
          <div
            aria-live="polite"
            style={{ maxHeight: "min(420px, 60vh)", overflowY: "auto" }}
          >
            {consulta.isError && (
              <ErroDoAto erro={consulta.error} ato="buscar" />
            )}
            {acoes ? (
              <ul
                className="col"
                style={{ gap: 2, listStyle: "none", margin: 0, padding: 0 }}
                aria-label="Telas"
              >
                {telas.map((t) => (
                  <li key={t.id}>
                    <button
                      type="button"
                      className="btn btn-g"
                      style={{ width: "100%", justifyContent: "flex-start" }}
                      onClick={() => {
                        abrir(t.href);
                      }}
                    >
                      {t.rotulo}
                    </button>
                  </li>
                ))}
                {telas.length === 0 && (
                  <li className="t13 mute">Nenhuma tela com esse nome.</li>
                )}
              </ul>
            ) : (
              <ul
                className="col"
                style={{ gap: 2, listStyle: "none", margin: 0, padding: 0 }}
                aria-label="Resultados"
              >
                {consulta.data?.map((r) => (
                  <li key={`${r.tipo}-${r.id}`}>
                    <button
                      type="button"
                      className="btn btn-g"
                      style={{
                        width: "100%",
                        justifyContent: "flex-start",
                        textAlign: "left",
                        height: "auto",
                        padding: "8px 10px",
                      }}
                      onClick={() => {
                        abrir(r.destino);
                      }}
                    >
                      <span style={{ display: "block" }}>
                        <span className="t13 mute">{NOME_DO_TIPO[r.tipo]}</span>
                        <b style={{ display: "block" }}>{r.titulo}</b>
                        <span className="t13 mute">{r.detalhe}</span>
                      </span>
                    </button>
                  </li>
                ))}
                {consulta.data?.length === 0 && (
                  <li className="t13 mute">
                    Nada encontrado para "{termo}". Confira a grafia; paciente
                    se busca também pelo nascimento (12/03/1968) ou celular.
                    Para telas, comece com "&gt;".
                  </li>
                )}
                {termo.length > 0 && termo.length < 3 && (
                  <li className="t13 mute">Digite pelo menos 3 letras.</li>
                )}
              </ul>
            )}
          </div>
          <p className="t13 mute" style={{ margin: 0 }}>
            Resultados conforme o seu perfil. Enter abre; Esc fecha.
          </p>
        </div>
      </Janela>
    </>
  );
}
