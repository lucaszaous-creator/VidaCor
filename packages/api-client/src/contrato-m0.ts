/**
 * Contrato HTTP v1 do esqueleto M0, escrito à mão a partir do docs/29 § 6.2.
 *
 * PROVISÓRIO: este arquivo será substituído pelos tipos gerados do `openapi.json` do núcleo
 * (`pnpm --filter @vidacor/api-client gerar`, que grava `src/gerado/openapi.ts`). Quando o arquivo
 * gerado existir, os tipos abaixo passam a ser apelidos dos gerados e qualquer diferença é bug do
 * contrato, resolvido no núcleo (o servidor é a fonte da verdade).
 *
 * Formatos conferidos com o `openapi.json` gerado pelo núcleo no commit 9b9e411 (branch do núcleo):
 * - datas de calendário (`data`, `nascimento`) em `AAAA-MM-DD` (`format: date`);
 * - instantes (`inicio`, `fim`, `criado_em`, `recebido_em`) em RFC 3339 (`format: date-time`);
 * - `trace_id` pode vir `null`; `atendimento_id` do item da agenda pode faltar ou vir `null`.
 */

/** `GET /v1/saude` (público). */
export interface Saude {
  situacao: 'ok';
}

export interface UsuarioDaSessao {
  id: string;
  nome: string;
  perfis: string[];
}

export interface UnidadeDaSessao {
  id: string;
  nome: string;
}

/** `GET /v1/sessao`. O `csrf` fica só em memória e vai no cabeçalho `X-CSRF-Token` das escritas. */
export interface Sessao {
  usuario: UsuarioDaSessao;
  unidade: UnidadeDaSessao;
  csrf: string;
}

/** `GET /v1/salas` (ato `agenda.ver`). */
export interface Sala {
  id: string;
  nome: string;
}

/** Sala da lista `/v1/salas`, com a lotação do usuário da sessão (FC-PUL-05). */
export interface SalaDaUnidade extends Sala {
  lotado: boolean;
}

/** Os dois identificadores do paciente (nome completo e nascimento). */
export interface PacienteResumo {
  id: string;
  nome: string;
  /** `AAAA-MM-DD`. */
  nascimento: string;
}

export interface ExameDoAgendamento {
  codigo_tuss: string;
  nome: string;
}

/** Situações do agendamento (docs/29 § 6.2). */
export const SITUACOES_AGENDAMENTO = [
  'agendado',
  'confirmado',
  'em_atendimento',
  'concluido',
  'faltou',
  'cancelado',
] as const;

export type SituacaoAgendamento = (typeof SITUACOES_AGENDAMENTO)[number];

export interface ItemDaAgenda {
  id: string;
  codigo: string;
  /** RFC 3339. */
  inicio: string;
  /** RFC 3339. */
  fim: string;
  sala: Sala;
  paciente: PacienteResumo;
  exame: ExameDoAgendamento;
  situacao: SituacaoAgendamento;
  /** Fora da lista `required` do openapi.json: pode faltar. */
  atendimento_id?: string | null;
}

/** `GET /v1/agenda?data=AAAA-MM-DD&sala_id=` (ato `agenda.ver`). */
export interface Agenda {
  data: string;
  itens: ItemDaAgenda[];
}

export interface ConsultaAgenda {
  /** `AAAA-MM-DD`. */
  data: string;
  sala_id?: string;
}

/** Corpo de `POST /v1/atendimentos` (ato `recepcao.checkin`, CSRF e `Idempotency-Key`). */
export interface NovoAtendimento {
  agendamento_id: string;
  /** O que a recepção conferiu com o paciente e o documento. Quem decide se confere é o servidor. */
  confirmacao: {
    nome: string;
    /** `AAAA-MM-DD`. */
    nascimento: string;
  };
}

/** Situações do atendimento, como descritas no openapi.json do núcleo. */
export const SITUACOES_ATENDIMENTO = ['aguardando_exame', 'em_exame', 'exame_concluido', 'cancelado'] as const;

export type SituacaoAtendimento = (typeof SITUACOES_ATENDIMENTO)[number];

/** Resposta 201 de `POST /v1/atendimentos`. */
export interface Atendimento {
  id: string;
  /** `ATD-AAAA-NNNNNN`, gerado pelo servidor. */
  codigo: string;
  /** `ACC-AAAA-NNNNNN`, gerado pelo servidor. */
  accession: string;
  agendamento_id: string;
  paciente: PacienteResumo;
  /** `aguardando_exame`, `em_exame`, `exame_concluido` ou `cancelado` (openapi.json). */
  situacao: SituacaoAtendimento;
  criado_em: string;
}

export interface EstudoRecebido {
  study_uid: string;
  imagens: number;
  recebido_em: string;
}

/** `GET /v1/atendimentos/{id}` (ato `recepcao.ver`, por objeto). */
export interface AtendimentoDetalhado extends Atendimento {
  estudo: EstudoRecebido | null;
}

/** Evento SSE `estudo-recebido` de `GET /v1/eventos`. */
export interface EventoEstudoRecebido {
  atendimento_id: string;
  accession: string;
  imagens: number;
}

/** Códigos de erro estáveis do check-in (docs/29 § 6.2). Outros códigos podem chegar do núcleo. */
export const CODIGOS_ERRO_CHECKIN = [
  'identificacao-nao-confere',
  'checkin-ja-realizado',
  'agendamento-nao-encontrado',
  'situacao-nao-permite-checkin',
  'idempotency-key-ausente',
] as const;

export type CodigoErroCheckin = (typeof CODIGOS_ERRO_CHECKIN)[number];

/** Corpo `application/problem+json` (RFC 9457) com as extensões do VidaCor. */
export interface ProblemaHttp {
  type: string;
  title: string;
  status: number;
  /** Mensagem em pt-BR no formato R8, escrita pelo servidor. */
  detail: string;
  /** Código estável (ex.: `identificacao-nao-confere`). */
  codigo: string;
  /** Código de suporte mostrado discretamente na tela (pode vir `null`). */
  trace_id?: string | null;
}
