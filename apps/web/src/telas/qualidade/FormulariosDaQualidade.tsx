import type {
  ClassificacaoDaNc,
  GravidadeDaNc,
  GravidadeDoEvento,
  NovaNaoConformidade,
  NovoEventoAdverso,
  PacienteResumo,
  TipoDeEvento,
} from "@vidacor/api-client";
import {
  AreaDeTexto,
  Aviso,
  Botao,
  CaixaDeMarcar,
  Escolha,
  formatarDataHora,
} from "@vidacor/design-system";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { EscolhaDePaciente } from "../../componentes/EscolhaDePaciente";
import { ErroDoAto } from "../../componentes/ErroDoAto";
import { useDependencias } from "../../contexto";
import { useOperacao } from "../../operacao";
import { FUSO_DA_CLINICA } from "../agenda/grade";

export const CLASSIFICACOES: { valor: ClassificacaoDaNc; rotulo: string }[] = [
  { valor: "processo", rotulo: "Processo" },
  { valor: "equipamento", rotulo: "Equipamento" },
  { valor: "documentacao", rotulo: "Documentação" },
  { valor: "fornecedor", rotulo: "Fornecedor" },
];
const GRAVIDADES_NC: { valor: GravidadeDaNc; rotulo: string }[] = [
  { valor: "leve", rotulo: "Leve" },
  { valor: "moderada", rotulo: "Moderada" },
  { valor: "grave", rotulo: "Grave" },
];
export const TIPOS_DE_EVENTO: { valor: TipoDeEvento; rotulo: string }[] = [
  { valor: "reacao_contraste", rotulo: "Reação a contraste" },
  { valor: "queda", rotulo: "Queda" },
  { valor: "erro_identificacao", rotulo: "Erro de identificação" },
  { valor: "exame_errado", rotulo: "Exame no paciente ou lado errado" },
  { valor: "falha_equipamento", rotulo: "Falha de equipamento" },
  { valor: "outro", rotulo: "Outro" },
];
export const GRAVIDADES_EVENTO: { valor: GravidadeDoEvento; rotulo: string }[] =
  [
    { valor: "sem_dano", rotulo: "Sem dano" },
    { valor: "leve", rotulo: "Dano leve" },
    { valor: "moderado", rotulo: "Dano moderado" },
    { valor: "grave", rotulo: "Dano grave" },
    { valor: "obito", rotulo: "Óbito" },
  ];

export function AbrirNaoConformidade() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [descricao, setDescricao] = useState("");
  const [classificacao, setClassificacao] = useState<string>("processo");
  const [gravidade, setGravidade] = useState<string>("leve");
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (n: NovaNaoConformidade, chave: string) =>
      api.qualidade.abrirNc(n, { idempotencyKey: chave }),
    {
      aoConcluir: () => cliente.invalidateQueries({ queryKey: ["qualidade"] }),
    },
  );
  if (op.resultado !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={`Não conformidade ${op.resultado.codigo} aberta.`}
        acoes={
          <Botao pequeno onPress={op.reiniciar}>
            Abrir outra
          </Botao>
        }
      >
        A gerência classifica, define o responsável e o prazo e acompanha até
        verificar a eficácia.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Abrir não conformidade"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (descricao.trim().length >= 10)
          op.enviar({
            descricao: descricao.trim(),
            classificacao: classificacao as ClassificacaoDaNc,
            gravidade: gravidade as GravidadeDaNc,
          });
      }}
    >
      <AreaDeTexto
        rotulo="O que aconteceu"
        value={descricao}
        onChange={setDescricao}
        linhas={3}
        erro={
          tentou && descricao.trim().length < 10
            ? "Descreva o que aconteceu (10 letras ou mais)."
            : null
        }
      />
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)",
          gap: 10,
          alignItems: "start",
        }}
      >
        <Escolha
          rotulo="Classificação"
          valor={classificacao}
          aoMudar={setClassificacao}
          opcoes={CLASSIFICACOES}
        />
        <Escolha
          rotulo="Gravidade da não conformidade"
          valor={gravidade}
          aoMudar={setGravidade}
          opcoes={GRAVIDADES_NC}
        />
      </div>
      {op.erro !== null && (
        <ErroDoAto erro={op.erro} ato="abrir a não conformidade" />
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Abrir não conformidade
        </Botao>
      </div>
    </form>
  );
}

export function NotificarEvento() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [tipo, setTipo] = useState<string>("reacao_contraste");
  const [quaseErro, setQuaseErro] = useState(false);
  const [gravidade, setGravidade] = useState<string>("sem_dano");
  const [descricao, setDescricao] = useState("");
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [anonimo, setAnonimo] = useState(false);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (n: NovoEventoAdverso, chave: string) =>
      api.qualidade.notificar(n, { idempotencyKey: chave }),
    {
      aoConcluir: () => cliente.invalidateQueries({ queryKey: ["qualidade"] }),
    },
  );
  if (op.resultado !== null) {
    const prazo = op.resultado.notificar_anvisa_ate;
    return (
      <Aviso
        nivel="feito"
        titulo={`Evento ${op.resultado.codigo} notificado ao Núcleo de Segurança do Paciente.`}
        acoes={
          <Botao pequeno onPress={op.reiniciar}>
            Notificar outro
          </Botao>
        }
      >
        {prazo === null || prazo === undefined
          ? "O Núcleo analisa e devolve o retorno."
          : `Dano grave: o Núcleo tem até ${formatarDataHora(prazo, FUSO_DA_CLINICA)} para notificar a Anvisa.`}
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Notificar evento ou quase-erro"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (descricao.trim().length >= 10)
          op.enviar({
            tipo: tipo as TipoDeEvento,
            quase_erro: quaseErro,
            gravidade: (quaseErro
              ? "sem_dano"
              : gravidade) as GravidadeDoEvento,
            descricao: descricao.trim(),
            paciente_id: paciente?.id ?? null,
            anonimo,
          });
      }}
    >
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "minmax(0, 1fr) minmax(0, 1fr)",
          gap: 10,
          alignItems: "start",
        }}
      >
        <Escolha
          rotulo="Tipo de evento"
          valor={tipo}
          aoMudar={setTipo}
          opcoes={TIPOS_DE_EVENTO}
        />
        <Escolha
          rotulo="Gravidade do dano"
          valor={quaseErro ? "sem_dano" : gravidade}
          aoMudar={setGravidade}
          opcoes={GRAVIDADES_EVENTO}
          desabilitado={quaseErro}
        />
      </div>
      <CaixaDeMarcar
        rotulo="Quase-erro (não chegou ao paciente)"
        marcado={quaseErro}
        aoMudar={setQuaseErro}
      />
      <EscolhaDePaciente
        rotulo="Paciente envolvido (opcional)"
        paciente={paciente}
        aoEscolher={setPaciente}
      />
      <AreaDeTexto
        rotulo="Descrição do evento"
        value={descricao}
        onChange={setDescricao}
        linhas={3}
        erro={
          tentou && descricao.trim().length < 10
            ? "Descreva o que aconteceu (10 letras ou mais)."
            : null
        }
      />
      <CaixaDeMarcar
        rotulo="Notificar de forma anônima"
        marcado={anonimo}
        aoMudar={setAnonimo}
        detalhe="O Núcleo não vê quem notificou."
      />
      {op.erro !== null && (
        <ErroDoAto erro={op.erro} ato="notificar o evento" />
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Enviar notificação
        </Botao>
      </div>
    </form>
  );
}
