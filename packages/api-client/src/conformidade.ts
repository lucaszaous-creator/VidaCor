/**
 * Conferência em tempo de compilação entre os tipos escritos à mão (que estreitam situações para
 * listas fechadas) e os tipos gerados do `openapi.json` do núcleo (`pnpm gerar`). Se o núcleo
 * renomear, tirar ou mudar o tipo de um campo, o `typecheck` quebra aqui antes de chegar à tela.
 * Valem os tipos da Fase 1, que são os usados pelas telas.
 */
import type { components } from './gerado/openapi';
import type { EstornoDetalhado, NovoPedidoDesconto, PedidoDesconto } from './contrato-pendencias';
import type { NovoAtendimento, PacienteResumo, Sala, SalaDaUnidade, Sessao } from './contrato-m0';
import type {
  AgendaFase1,
  AtendimentoFase1,
  Caixa,
  CorrecaoCadastro,
  Etiqueta,
  EstudoPendente,
  FilaDaSala,
  Guia,
  ItemDaAgendaFase1,
  ItemDaFilaDeLaudos,
  Laudo,
  LaudoDoPortal,
  LinkCriado,
  PedidoAgendamento,
  PedidoGuia,
  PedidoRecebimento,
  PedidoTriagem,
  Recebimento,
  Reimpressao,
  ResultadoDoBipe,
  Triagem,
} from './contrato-fase1';

type S = components['schemas'];
type Cabe<A extends B, B> = A;

export type ConfereSala = Cabe<Sala, S['Sala']>;
export type ConfereSalaDaUnidade = Cabe<SalaDaUnidade, S['SalaDaUnidade']>;
export type ConferePaciente = Cabe<PacienteResumo, S['PacienteResumo']>;
export type ConfereSessao = Cabe<Sessao, S['SessaoAtual']>;
export type ConferePedidoCheckin = Cabe<NovoAtendimento, S['PedidoCheckin']>;
export type ConfereItemDaAgenda = Cabe<ItemDaAgendaFase1, S['ItemAgenda']>;
export type ConfereAgenda = Cabe<AgendaFase1, S['AgendaDoDia']>;
export type ConfereAtendimento = Cabe<AtendimentoFase1, S['Atendimento']>;
export type ConfereEtiqueta = Cabe<Etiqueta, S['Etiqueta']>;
export type ConfereReimpressao = Cabe<Reimpressao, S['Reimpressao']>;
export type ConfereCorrecao = Cabe<CorrecaoCadastro, S['Correcao']>;
export type ConfereBipe = Cabe<ResultadoDoBipe, S['ResultadoBipe']>;
export type ConfereFila = Cabe<FilaDaSala, S['FilaDaSala']>;
export type ConfereTriagem = Cabe<Triagem, S['Triagem']>;
export type ConferePedidoTriagem = Cabe<PedidoTriagem, S['PedidoTriagem']>;
export type ConfereGuia = Cabe<Guia, S['Guia']>;
export type ConferePedidoGuia = Cabe<PedidoGuia, S['PedidoGuia']>;
export type ConferePedidoAgendamento = Cabe<PedidoAgendamento, S['PedidoAgendamento']>;
export type ConfereLaudo = Cabe<Laudo, S['Laudo']>;
export type ConfereFilaDeLaudos = Cabe<ItemDaFilaDeLaudos, S['ItemFilaLaudo']>;
export type ConferePortal = Cabe<LaudoDoPortal, S['LaudoDoPortal']>;
export type ConfereLink = Cabe<LinkCriado, S['LinkCriado']>;
export type ConfereEstudoPendente = Cabe<EstudoPendente, S['EstudoPendente']>;
export type ConfereCaixa = Cabe<Caixa, S['Caixa']>;
export type ConfereRecebimento = Cabe<Recebimento, S['Recebimento']>;
export type ConferePedidoRecebimento = Cabe<PedidoRecebimento, S['PedidoRecebimento']>;

// Pendências da gerência no caixa.
export type ConferePedidoDesconto = Cabe<PedidoDesconto, S['PedidoDesconto']>;
export type ConfereNovoPedidoDesconto = Cabe<NovoPedidoDesconto, S['NovoPedidoDesconto']>;
export type ConfereEstornoDetalhado = Cabe<EstornoDetalhado, S['EstornoDetalhado']>;
