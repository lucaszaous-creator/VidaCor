/**
 * Conferência em tempo de compilação: os tipos de `contrato-m0.ts` precisam caber nos tipos gerados do
 * `openapi.json` do núcleo (`src/gerado/openapi.ts`, `pnpm gerar`). Se o núcleo renomear ou tirar um
 * campo, o `typecheck` quebra aqui. Os tipos escritos à mão continuam porque estreitam campos que o
 * contrato ainda descreve como texto livre (situações); quando o núcleo publicar esses campos como
 * enum no OpenAPI, eles passam a ser apelidos diretos dos gerados.
 */
import type { components } from './gerado/openapi';
import type {
  Agenda,
  AtendimentoDetalhado,
  ItemDaAgenda,
  NovoAtendimento,
  PacienteResumo,
  Sala,
  Sessao,
} from './contrato-m0';

type S = components['schemas'];
type Cabe<A extends B, B> = A;

export type ConfereSala = Cabe<Sala, S['Sala']>;
export type ConferePaciente = Cabe<PacienteResumo, S['PacienteResumo']>;
export type ConfereItem = Cabe<Required<ItemDaAgenda>, S['ItemAgenda']>;
export type ConfereAgenda = Cabe<{ data: Agenda['data']; itens: Required<ItemDaAgenda>[] }, S['AgendaDoDia']>;
export type ConfereSessao = Cabe<Sessao, S['SessaoAtual']>;
export type ConferePedido = Cabe<NovoAtendimento, S['PedidoCheckin']>;
export type ConfereAtendimento = Cabe<AtendimentoDetalhado, S['Atendimento']>;
