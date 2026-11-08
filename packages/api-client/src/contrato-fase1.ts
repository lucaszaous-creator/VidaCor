/**
 * Contrato HTTP da Fase 1 (os 10 fluxos críticos): docs/30, incluindo a § 7 (ajustes feitos na
 * implementação), alinhado aos schemas do `openapi.json` gerado pelo núcleo no commit 97377fd.
 *
 * Os tipos ficam escritos à mão porque estreitam o que o OpenAPI ainda publica como texto livre
 * (situações, níveis). Quando `src/gerado/openapi.ts` for regenerado a partir desse `openapi.json`
 * (arquivo gerado, fora do escopo das telas), `src/conformidade.ts` passa a conferir também estes
 * tipos. Respostas que o OpenAPI ainda não descreve (estorno, ficha de contingência, acesso de
 * emergência, conciliação, números e resolução) estão marcadas "A CONFIRMAR".
 *
 * Convenções do docs/30 § 0: datas `AAAA-MM-DD`; instantes RFC 3339; dinheiro em centavos (inteiro);
 * percentuais em pontos base (`500` = 5,00%); situações e motivos sempre por palavra.
 */
import type { ItemDaAgenda, PacienteResumo, Sala } from './contrato-m0';

/* ------------------------------------------------------------------------------------------------
 * 1. Recepção, etiquetas e sala (FC-CHK, FC-ETQ, FC-PUL)
 * ---------------------------------------------------------------------------------------------- */

/** Corpo de `POST /v1/atendimentos` (schema `PedidoCheckin`). */
export interface PedidoCheckin {
  agendamento_id: string;
  /** O que a recepção conferiu. Quem decide se confere com o cadastro é o servidor (G1). */
  confirmacao: {
    nome: string;
    /** `AAAA-MM-DD`. */
    nascimento: string;
    documento_com_foto: boolean;
  };
}

export const SITUACOES_ETIQUETA = ['valida', 'invalidada'] as const;
export type SituacaoEtiqueta = (typeof SITUACOES_ETIQUETA)[number];

/** Etiqueta (schema `Etiqueta`). Leva só nome, nascimento e códigos; nunca diagnóstico (docs/20). */
export interface Etiqueta {
  /** `ETQ-AAAA-NNNNNN`. */
  codigo: string;
  tipo: string;
  situacao: SituacaoEtiqueta;
  motivo_invalidacao?: string | null;
  /** Código da etiqueta que substituiu esta, quando invalidada por reimpressão. */
  substituida_por?: string | null;
}

/** Situações do atendimento (o OpenAPI publica como texto). */
export const SITUACOES_ATENDIMENTO_FASE1 = ['aguardando_sala', 'aguardando_exame', 'em_exame', 'exame_concluido', 'cancelado'] as const;
export type SituacaoAtendimentoFase1 = (typeof SITUACOES_ATENDIMENTO_FASE1)[number];

export interface EstudoResumo {
  study_uid: string;
  imagens: number;
  recebido_em: string;
}

/** Atendimento (schema `Atendimento`): resposta do check-in e de `GET /v1/atendimentos/{id}`. */
export interface AtendimentoFase1 {
  id: string;
  codigo: string;
  accession: string;
  agendamento_id: string;
  paciente: PacienteResumo;
  sala: Sala;
  situacao: SituacaoAtendimentoFase1;
  requer_contraste: boolean;
  /** Senha da fila (ex.: `A012`); `null` antes de emitida. */
  senha?: string | null;
  /** Pulseira válida do atendimento; `null` quando todas foram invalidadas (correção de cadastro). */
  pulseira?: Etiqueta | null;
  criado_em: string;
  estudo?: EstudoResumo | null;
  triagem?: Triagem | null;
  /** Só a situação; o conteúdo do laudo a recepção não vê (FC-ACE-06). */
  laudo_situacao?: string | null;
}

/** Mesmo schema; mantido o nome para as telas que leem o atendimento. */
export type AtendimentoDetalhadoFase1 = AtendimentoFase1;

export type SituacaoAgendamentoFase1 =
  | 'agendado'
  | 'confirmado'
  | 'reserva_provisoria'
  | 'expirada'
  | 'em_atendimento'
  | 'concluido'
  | 'faltou'
  | 'cancelado';

/** Campos que a Fase 1 acrescenta ao item de `GET /v1/agenda` (schema `ItemAgenda`). */
export interface ExtrasDoItemDaAgenda {
  /** Outros pacientes do dia com o mesmo nome e outro nascimento (alerta de homônimo, FC-CHK-04). */
  homonimos: PacienteResumo[];
  requer_contraste: boolean;
  laudo_situacao?: string | null;
  /** Prazo da reserva provisória (RFC 3339). */
  expira_em?: string | null;
}

/** Item de `GET /v1/agenda` e resposta de `POST /v1/agendamentos` (schema `ItemAgenda`). */
export interface ItemDaAgendaFase1 extends Omit<ItemDaAgenda, 'situacao'>, ExtrasDoItemDaAgenda {
  situacao: SituacaoAgendamentoFase1;
}

export interface AgendaFase1 {
  data: string;
  itens: ItemDaAgendaFase1[];
}

/** `POST /v1/etiquetas/{codigo}/reimpressao`. */
export interface PedidoReimpressao {
  motivo: string;
}

export interface Reimpressao {
  nova: Etiqueta;
  invalidada: Etiqueta;
}

/** `POST /v1/pacientes/{id}/correcao` (reautenticação). */
export interface PedidoCorrecaoCadastro {
  /** `AAAA-MM-DD`. */
  nascimento: string;
  motivo: string;
}

export interface CorrecaoCadastro {
  paciente: PacienteResumo;
  versao: number;
  /** Códigos das etiquetas invalidadas; a tela pede a reimpressão. */
  etiquetas_invalidadas: string[];
}

/** `POST /v1/salas/{sala_id}/bipes` (exige `Idempotency-Key`). */
export interface PedidoBipe {
  codigo_etiqueta: string;
}

export interface Aparelho {
  nome: string;
  ae_title: string;
}

export interface ResultadoDoBipe {
  atendimento: AtendimentoFase1;
  aparelho: Aparelho;
  worklist: 'enviada' | 'ja_enviada';
}

/** Item de `GET /v1/salas/{sala_id}/fila` (schema `ItemFila`). */
export interface ItemDaFilaDaSala {
  atendimento_id: string;
  codigo: string;
  paciente: PacienteResumo;
  /** Nome do exame. */
  exame: string;
  /** RFC 3339. */
  inicio: string;
  situacao: string;
  senha?: string | null;
  requer_contraste: boolean;
  triagem_resultado?: string | null;
}

export interface FilaDaSala {
  sala: Sala;
  itens: ItemDaFilaDaSala[];
}

/* ------------------------------------------------------------------------------------------------
 * 2. Triagem com contraste (FC-TRI)
 * ---------------------------------------------------------------------------------------------- */

/** Corpo de `POST /v1/atendimentos/{id}/triagem`. O servidor ignora `tfg` e calcula (FC-TRI-07). */
export interface PedidoTriagem {
  reacao_previa_contraste: boolean;
  gestante: boolean;
  amamentando: boolean;
  usa_metformina: boolean;
  /** Texto decimal com ponto (`"1.4"`). */
  creatinina_mg_dl: string;
  /** `AAAA-MM-DD`. */
  creatinina_data: string;
}

export const RESULTADOS_TRIAGEM = ['liberado', 'liberado_com_ressalvas', 'precisa_aprovacao_medica'] as const;
export type ResultadoDaTriagem = (typeof RESULTADOS_TRIAGEM)[number];

export interface Conduta {
  texto: string;
  medico_id: string;
  registrada_em: string;
}

/** Schema `Triagem`. */
export interface Triagem {
  codigo: string;
  resultado: ResultadoDaTriagem;
  /** O servidor diz se o exame pode iniciar (com a conduta, quando exigida). */
  exame_liberado: boolean;
  /** TFG calculada pelo servidor (CKD-EPI 2021), em mL/min/1,73 m². */
  tfg_calculada?: number | null;
  motivos: string[];
  /** Versão da regra aplicada (ex.: `TRI-2026.1`). */
  regra_versao: string;
  conduta?: Conduta | null;
}

/** `POST /v1/atendimentos/{id}/triagem/conduta` (médico com vínculo, reautenticação). */
export interface PedidoConduta {
  conduta: string;
}

/* ------------------------------------------------------------------------------------------------
 * 3. Agendamento de convênio com guia (FC-AGD)
 * ---------------------------------------------------------------------------------------------- */

export interface ProcedimentoPedido {
  tuss: string;
  quantidade: number;
}

/** `POST /v1/guias`. */
export interface PedidoGuia {
  operadora: string;
  numero: string;
  carteirinha: string;
  senha: string;
  /** `AAAA-MM-DD`. */
  validade: string;
  paciente_id: string;
  solicitante: string;
  procedimentos: ProcedimentoPedido[];
}

/** Schema `Guia`. Saldo por procedimento conferido pelo servidor. */
export interface Guia {
  id: string;
  codigo: string;
  numero: string;
  operadora: string;
  validade: string;
  situacao: string;
  procedimentos: { tuss: string; autorizado: number; saldo: number }[];
  /** Atendimento que está usando a guia, quando em uso. */
  em_uso_por?: string | null;
}

/** `POST /v1/agendamentos`. */
export interface PedidoAgendamento {
  paciente_id: string;
  sala_id: string;
  /** RFC 3339. */
  inicio: string;
  procedimentos: string[];
  guia_id: string | null;
  reserva_provisoria: boolean;
}

/** Resposta de `POST /v1/agendamentos` e `POST /v1/agendamentos/{id}/guia` (schema `ItemAgenda`). */
export type AgendamentoCriado = ItemDaAgendaFase1;

/** `GET /v1/salas/{id}/horarios-livres?data=`: inícios (RFC 3339) dos horários livres de 30 minutos. */
export type HorarioLivre = string;

/* ------------------------------------------------------------------------------------------------
 * 4. Laudo, portal, links e estudos (FC-LAU, FC-ACE)
 * ---------------------------------------------------------------------------------------------- */

export const SITUACOES_LAUDO = ['rascunho', 'assinado', 'retificacao_em_rascunho'] as const;
export type SituacaoLaudo = (typeof SITUACOES_LAUDO)[number];

export type SituacaoVersao = 'rascunho' | 'assinada' | 'substituida';

/** Schema `Versao`. O texto é puro: a tela mostra como texto, nunca como HTML (SEC-CLI-1). */
export interface VersaoDoLaudo {
  numero: number;
  texto: string;
  situacao: SituacaoVersao;
  motivo?: string | null;
  /** Identificador de quem assinou. */
  assinada_por?: string | null;
  assinada_em?: string | null;
  hash_documento?: string | null;
}

/** Schema `Laudo`. */
export interface Laudo {
  id: string;
  codigo: string;
  paciente: PacienteResumo;
  /** Nome do exame. */
  exame: string;
  situacao: SituacaoLaudo;
  conversar_antes: boolean;
  publicado_no_portal: boolean;
  versao_vigente?: number | null;
  versoes: VersaoDoLaudo[];
  estudo?: EstudoResumo | null;
  medico_atribuido_id?: string | null;
}

/** Item de `GET /v1/laudos/fila` (schema `ItemFilaLaudo`). */
export interface ItemDaFilaDeLaudos {
  id: string;
  codigo: string;
  paciente: PacienteResumo;
  exame: string;
  situacao: SituacaoLaudo;
  conversar_antes: boolean;
  recebido_em: string;
}

export interface PedidoRascunho {
  texto: string;
  versao_numero: number;
}

export interface PedidoAssinatura {
  pin: string;
}

export interface PedidoRetificacao {
  texto: string;
  motivo: string;
}

export interface PedidoConversarAntes {
  marcado: boolean;
}

export interface PedidoAcessoEmergencia {
  /** Mínimo de 20 caracteres (conferido pelo servidor). */
  justificativa: string;
}

/** A CONFIRMAR: o OpenAPI não descreve a resposta; o docs/30 diz só "vale 1 hora". */
export interface AcessoDeEmergencia {
  valido_ate?: string;
}

/** `POST /v1/laudos/{id}/links` `{ horas }`: o token vem uma única vez. */
export interface PedidoLink {
  horas: number;
}

export interface LinkCriado {
  token: string;
  expira_em: string;
}

/** Laudo como o paciente vê no portal (schema `LaudoDoPortal`): só a versão vigente. */
export interface LaudoDoPortal {
  id: string;
  exame: string;
  publicado_em: string;
  /** Verdadeiro quando a versão vigente substituiu outra (FC-LAU-05). */
  retificado: boolean;
  texto: string;
}

/** `GET /v1/portal/laudos` devolve a mesma forma. */
export type ItemDoPortal = LaudoDoPortal;

/** Estudo recebido sem conciliação (schema `EstudoPendente`). */
export interface EstudoPendente {
  id: string;
  codigo: string;
  accession?: string | null;
  imagens: number;
  recebido_em: string;
  pendencia?: string | null;
}

/** `POST /v1/estudos/{id}/conciliacao`: os dois identificadores conferidos pelo médico. */
export interface PedidoConciliacao {
  atendimento_id: string;
  nome: string;
  /** `AAAA-MM-DD`. */
  nascimento: string;
}

/* ------------------------------------------------------------------------------------------------
 * 5. Caixa e PIX (FC-CXA, FC-PIX)
 * ---------------------------------------------------------------------------------------------- */

/** Schema `Caixa`. No fechamento, o servidor preenche esperado, contado e diferença. */
export interface Caixa {
  id: string;
  codigo: string;
  aberto_em: string;
  fundo_centavos: number;
  fechado_em?: string | null;
  esperado_centavos?: number | null;
  contado_centavos?: number | null;
  diferenca_centavos?: number | null;
}

export type FormaDePagamento = 'pix' | 'dinheiro' | 'cartao';

export interface DescontoPedido {
  pontos_base: number;
  motivo: string;
  autorizacao_id?: string;
}

/** `POST /v1/recebimentos`. A tela não manda total. */
export interface PedidoRecebimento {
  paciente_id: string;
  atendimento_id: string;
  itens: { tuss: string; valor_centavos?: number }[];
  desconto: DescontoPedido | null;
  forma: FormaDePagamento;
}

export type SituacaoRecebimento = 'aguardando_pagamento' | 'pago' | 'estornado';

/** Schema `Recebimento`. `pix` vem `null` até o PSP devolver a cobrança. */
export interface Recebimento {
  id: string;
  codigo: string;
  forma: string;
  itens: { tuss: string; descricao: string; valor_centavos: number }[];
  subtotal_centavos: number;
  desconto_centavos: number;
  total_centavos: number;
  tabela_versao: number;
  situacao: string;
  pix?: { txid: string; copia_e_cola: string } | null;
}

/** `POST /v1/autorizacoes-desconto` (gerência, reautenticação). */
export interface PedidoAutorizacaoDesconto {
  paciente_id: string;
  pontos_base: number;
  motivo: string;
  solicitante_id: string;
}

export interface AutorizacaoDeDesconto {
  id: string;
  codigo: string;
  valida_ate: string;
}

export interface PedidoEstorno {
  motivo: string;
}

/** A CONFIRMAR: o OpenAPI não descreve a forma do estorno. */
export interface Estorno {
  id: string;
  situacao?: string;
  motivo?: string;
}

/* ------------------------------------------------------------------------------------------------
 * 6. Contingência (FC-CTG)
 * ---------------------------------------------------------------------------------------------- */

export const NIVEIS_CONTINGENCIA = ['externo', 'setor', 'total'] as const;
export type NivelDeContingencia = (typeof NIVEIS_CONTINGENCIA)[number];

export interface PedidoContingencia {
  nivel: NivelDeContingencia;
  motivo: string;
  responsavel_id: string;
  /** RFC 3339: a hora original, gravada na estação (FC-CTG-02). */
  declarada_em: string;
  /** UUID gerado na estação; repetir não cria outra. */
  chave: string;
}

/** Schema `Contingencia`. */
export interface Contingencia {
  id: string;
  codigo: string;
  nivel: string;
  motivo: string;
  responsavel_id: string;
  declarada_por: string;
  declarada_em: string;
  encerrada_em?: string | null;
  situacao: string;
}

/** `POST /v1/contingencias/{id}/fichas` (§ 7: leva `agendamento_id`; `atendido_por` é identificador). */
export interface PedidoFicha {
  formulario: 'F-ATD';
  numero: number;
  agendamento_id: string;
  /** Nome como está no papel. */
  nome: string;
  /** Nascimento como está no papel (`AAAA-MM-DD`). */
  nascimento: string;
  ocorrido_em: string;
  atendido_por: string;
  anexo_sha256: string;
}

export interface PedidoRecibo {
  numero: number;
  paciente_id: string;
  tuss: string;
  valor_papel_centavos: number;
  ocorrido_em: string;
}

/** `POST /v1/contingencias/{id}/numeros`: números da faixa que voltaram sem uso ou foram anulados. */
export interface PedidoNumeros {
  formulario: 'F-ATD' | 'F-REC';
  de: number;
  ate: number;
  situacao: 'devolvido' | 'anulado';
}

/** `POST /v1/divergencias/{id}/resolucao` (gerência). */
export interface PedidoResolucao {
  resolucao: string;
}

/** A CONFIRMAR: forma da divergência aberta (respostas 202 e recibo com preço diferente). */
export interface Divergencia {
  id?: string;
  codigo: string;
  tipo?: string;
  descricao?: string;
}

/** Resposta de `POST .../fichas`: 201 com o atendimento ou 202 com a divergência aberta. A CONFIRMAR. */
export type ResultadoDaFicha =
  | { situacao: 'lancada'; atendimento: AtendimentoFase1 }
  | { situacao: 'divergencia-aberta'; divergencia: Divergencia };

/** A CONFIRMAR: forma da resposta de `POST .../recibos`. */
export interface ReciboLancado {
  codigo: string;
  valor_papel_centavos: number;
  valor_recalculado_centavos: number;
  divergencia: Divergencia | null;
}

/* ------------------------------------------------------------------------------------------------
 * Códigos de erro estáveis da Fase 1 (docs/30, inclusive § 7). Outros podem chegar do núcleo.
 * ---------------------------------------------------------------------------------------------- */

export const CODIGO_REAUTENTICACAO = 'reautenticacao-necessaria';

export const CODIGOS_ERRO_FASE1 = [
  'reautenticacao-necessaria',
  'sem-permissao',
  'identificacao-incompleta',
  'identificacao-nao-confere',
  'checkin-ja-realizado',
  'situacao-nao-permite-checkin',
  'motivo-obrigatorio',
  'etiqueta-ja-invalidada',
  'etiqueta-substituida',
  'paciente-de-outra-sala',
  'fora-da-sala',
  'sala-sem-aparelho',
  'triagem-ausente',
  'conduta-medica-pendente',
  'cadastro-sem-sexo',
  'etiqueta-nao-encontrada',
  'guia-em-uso',
  'procedimento-nao-autorizado',
  'procedimento-desconhecido',
  'senha-vencida',
  'saldo-esgotado',
  'horario-ocupado',
  'sem-vinculo-de-cuidado',
  'laudo-assinado',
  'laudo-de-outro-medico',
  'assinatura-concorrente',
  'assinatura-indisponivel',
  'pin-recusado',
  'laudo-nao-assinado',
  'nao-encontrado',
  'link-expirado',
  'desconto-acima-do-limite',
  'valor-invalido',
  'caixa-fechado',
  'sem-tabela-de-precos',
  'separacao-de-funcoes',
  'ficha-ja-lancada',
  'numero-fora-da-faixa',
  'divergencia-aberta',
  'pendencias',
] as const;

export type CodigoErroFase1 = (typeof CODIGOS_ERRO_FASE1)[number];
