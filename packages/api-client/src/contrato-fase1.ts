/**
 * Contrato HTTP da Fase 1 (os 10 fluxos críticos), escrito à mão a partir do docs/30.
 *
 * PROVISÓRIO, como o `contrato-m0.ts`: será CONFERIDO contra os tipos gerados do `openapi.json` do
 * núcleo (`src/gerado/openapi.ts`) assim que o núcleo publicar estas rotas; a partir daí qualquer
 * diferença é bug do contrato, resolvido no núcleo (o servidor é a fonte da verdade). Campos que o
 * docs/30 não descreve e que a tela precisou supor estão marcados com "A CONFIRMAR" e listados na
 * entrega para o núcleo decidir.
 *
 * Convenções do docs/30 § 0: datas `AAAA-MM-DD`; instantes RFC 3339; dinheiro em centavos (inteiro);
 * percentuais em pontos base (`500` = 5,00%); situações e motivos sempre por palavra.
 */
import type { ItemDaAgenda, PacienteResumo, Sala } from './contrato-m0';

/* ------------------------------------------------------------------------------------------------
 * 1. Recepção, etiquetas e sala (FC-CHK, FC-ETQ, FC-PUL)
 * ---------------------------------------------------------------------------------------------- */

/** Corpo de `POST /v1/atendimentos` na Fase 1 (substitui o corpo do M0). */
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

/** Etiqueta (pulseira). Leva só nome, nascimento e códigos; nunca diagnóstico (docs/20). */
export interface Etiqueta {
  /** `ETQ-AAAA-NNNNNN`. */
  codigo: string;
  tipo: 'pulseira';
  situacao: SituacaoEtiqueta;
  motivo_invalidacao: string | null;
  /** Código da etiqueta que substituiu esta, quando invalidada por reimpressão. */
  substituida_por: string | null;
}

/** Pulseira como vem no atendimento: o mínimo do docs/30 (`codigo` e `situacao`). */
export interface PulseiraDoAtendimento {
  codigo: string;
  situacao: SituacaoEtiqueta;
}

/** Situações do atendimento na Fase 1 (docs/30 § 1 acrescenta `aguardando_sala`). */
export const SITUACOES_ATENDIMENTO_FASE1 = [
  'aguardando_sala',
  'aguardando_exame',
  'em_exame',
  'exame_concluido',
  'cancelado',
] as const;
export type SituacaoAtendimentoFase1 = (typeof SITUACOES_ATENDIMENTO_FASE1)[number];

/** Resposta 201 do check-in da Fase 1: com senha e pulseira. */
export interface AtendimentoFase1 {
  id: string;
  codigo: string;
  accession: string;
  agendamento_id: string;
  paciente: PacienteResumo;
  situacao: SituacaoAtendimentoFase1;
  /** Senha da fila (ex.: `A012`). */
  senha: string;
  pulseira: PulseiraDoAtendimento;
  criado_em: string;
  /** `contingencia` quando lançado de ficha em papel (docs/30 § 6). A CONFIRMAR: ausente fora dela. */
  origem?: 'sistema' | 'contingencia';
}

/** Situações do laudo como a recepção vê (sem conteúdo, FC-ACE-06). */
export type SituacaoDoLaudo = 'rascunho' | 'assinado' | 'retificacao_em_rascunho';

/** `GET /v1/atendimentos/{id}` na Fase 1. */
export interface AtendimentoDetalhadoFase1 extends AtendimentoFase1 {
  estudo: { study_uid: string; imagens: number; recebido_em: string } | null;
  triagem: Triagem | null;
  /** Só a situação; o conteúdo do laudo a recepção não vê. */
  laudo_situacao: SituacaoDoLaudo | null;
}

/** Campos que a Fase 1 acrescenta ao item de `GET /v1/agenda`. */
export interface ExtrasDoItemDaAgenda {
  /** Outros pacientes do dia com o mesmo nome e outro nascimento (alerta de homônimo, FC-CHK-04). */
  homonimos: PacienteResumo[];
  requer_contraste: boolean;
  laudo_situacao: SituacaoDoLaudo | null;
}

/** Item de `GET /v1/agenda` na Fase 1. */
export interface ItemDaAgendaFase1 extends ItemDaAgenda, ExtrasDoItemDaAgenda {}

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

/** `POST /v1/salas/{sala_id}/bipes`. */
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

/**
 * Item de `GET /v1/salas/{sala_id}/fila`. O docs/30 diz só "atendimentos do dia da sala com
 * situação". A CONFIRMAR: a forma abaixo (horário, exame, contraste e resultado da triagem).
 */
export interface ItemDaFilaDaSala {
  atendimento_id: string;
  codigo: string;
  paciente: PacienteResumo;
  exame: { codigo_tuss: string; nome: string };
  /** RFC 3339. */
  inicio: string;
  situacao: SituacaoAtendimentoFase1;
  requer_contraste: boolean;
  triagem_resultado: ResultadoDaTriagem | null;
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
  /** Texto decimal com ponto (`"1.4"`), como o docs/30. */
  creatinina_mg_dl: string;
  /** `AAAA-MM-DD`. */
  creatinina_data: string;
  /** Opcional e ignorado pelo servidor; a tela não calcula nem envia. */
  tfg?: number;
}

export const RESULTADOS_TRIAGEM = ['liberado', 'liberado_com_ressalvas', 'precisa_aprovacao_medica'] as const;
export type ResultadoDaTriagem = (typeof RESULTADOS_TRIAGEM)[number];

export interface Conduta {
  texto: string;
  medico: string;
  registrada_em: string;
}

export interface Triagem {
  resultado: ResultadoDaTriagem;
  /** TFG calculada pelo servidor (CKD-EPI 2021), em mL/min/1,73 m². */
  tfg_calculada: number | null;
  motivos: string[];
  /** Versão da regra aplicada (ex.: `TRI-2026.1`). */
  regra_versao: string;
  /** A CONFIRMAR: o docs/30 diz só "null | Conduta"; os campos de `Conduta` são suposição da tela. */
  conduta: Conduta | null;
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

export interface Guia {
  id: string;
  numero: string;
  operadora: string;
  validade: string;
  situacao: 'disponivel' | 'consumida';
  /** Saldo por procedimento, conferido pelo servidor. */
  procedimentos: { tuss: string; saldo: number }[];
  /** Atendimento que consumiu a guia, quando consumida. */
  atendimento_codigo: string | null;
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

export type SituacaoAgendamentoFase1 =
  | 'agendado'
  | 'confirmado'
  | 'reserva_provisoria'
  | 'expirada'
  | 'em_atendimento'
  | 'concluido'
  | 'faltou'
  | 'cancelado';

/** `ItemAgenda` devolvido por `POST /v1/agendamentos`. */
export interface AgendamentoCriado {
  id: string;
  codigo: string;
  inicio: string;
  fim: string;
  sala: Sala;
  paciente: PacienteResumo;
  situacao: SituacaoAgendamentoFase1;
  /** Prazo da reserva provisória (RFC 3339), quando `reserva_provisoria`. */
  expira_em?: string | null;
}

/** `GET /v1/salas/{id}/horarios-livres?data=`. A CONFIRMAR: forma de cada horário. */
export interface HorarioLivre {
  inicio: string;
  fim: string;
}

/* ------------------------------------------------------------------------------------------------
 * 4. Laudo, portal e acesso (FC-LAU, FC-ACE)
 * ---------------------------------------------------------------------------------------------- */

export const SITUACOES_LAUDO = ['rascunho', 'assinado', 'retificacao_em_rascunho'] as const;
export type SituacaoLaudo = (typeof SITUACOES_LAUDO)[number];

export type SituacaoVersao = 'rascunho' | 'assinada' | 'substituida';

export interface VersaoDoLaudo {
  numero: number;
  /** Texto puro. A tela mostra como texto, nunca como HTML (SEC-CLI-1). */
  texto: string;
  situacao: SituacaoVersao;
  motivo: string | null;
  assinada_por: string | null;
  assinada_em: string | null;
  hash_documento: string | null;
}

/** A CONFIRMAR: o docs/30 cita `exame` e `estudo` sem forma. */
export interface ExameDoLaudo {
  codigo_tuss: string;
  nome: string;
}

export interface EstudoDoLaudo {
  accession: string;
  imagens: number;
}

export interface Laudo {
  id: string;
  codigo: string;
  paciente: PacienteResumo;
  exame: ExameDoLaudo;
  situacao: SituacaoLaudo;
  conversar_antes: boolean;
  publicado_no_portal: boolean;
  versao_vigente: number;
  versoes: VersaoDoLaudo[];
  estudo: EstudoDoLaudo | null;
}

/** Item de `GET /v1/laudos/fila`. A CONFIRMAR: o docs/30 não descreve a forma. */
export interface ItemDaFilaDeLaudos {
  id: string;
  codigo: string;
  paciente: PacienteResumo;
  exame: ExameDoLaudo;
  situacao: SituacaoLaudo;
  conversar_antes: boolean;
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

/** A CONFIRMAR: o docs/30 diz só "vale 1 hora". */
export interface AcessoDeEmergencia {
  valido_ate: string;
}

/** Laudo como o paciente vê no portal: só a versão vigente, com a marca de retificação. */
export interface LaudoDoPortal {
  id: string;
  codigo: string;
  exame: ExameDoLaudo;
  /** A CONFIRMAR: data de assinatura da versão vigente. */
  assinado_em: string;
  medico: string;
  /** Verdadeiro quando a versão vigente substituiu outra (FC-LAU-05). */
  retificado: boolean;
  versao: number;
  texto: string;
}

/** Item da lista `GET /v1/portal/laudos` (A CONFIRMAR: sem o texto). */
export type ItemDoPortal = Omit<LaudoDoPortal, 'texto'>;

/* ------------------------------------------------------------------------------------------------
 * 5. Caixa e PIX (FC-CXA, FC-PIX)
 * ---------------------------------------------------------------------------------------------- */

/** A CONFIRMAR: o docs/30 não descreve a forma do caixa. */
export interface Caixa {
  id: string;
  codigo: string;
  aberto_em: string;
  fundo_centavos: number;
  situacao: 'aberto' | 'fechado';
}

export interface FechamentoDeCaixa {
  esperado_centavos: number;
  contado_centavos: number;
  diferenca_centavos: number;
}

export type FormaDePagamento = 'pix' | 'dinheiro' | 'cartao';

export interface DescontoPedido {
  pontos_base: number;
  motivo: string;
  autorizacao_id?: string;
}

/** `POST /v1/recebimentos`. A tela não manda total; `valor_centavos` só quando o item não tem preço de tabela. */
export interface PedidoRecebimento {
  paciente_id: string;
  atendimento_id: string;
  itens: { tuss: string; valor_centavos?: number }[];
  desconto: DescontoPedido | null;
  forma: FormaDePagamento;
}

export type SituacaoRecebimento = 'aguardando_pagamento' | 'pago' | 'estornado';

export interface Recebimento {
  id: string;
  codigo: string;
  itens: { tuss: string; descricao: string; valor_centavos: number }[];
  subtotal_centavos: number;
  desconto_centavos: number;
  total_centavos: number;
  tabela_versao: string;
  situacao: SituacaoRecebimento;
  pix: { txid: string; copia_e_cola: string } | null;
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
  valida_ate: string;
}

export interface PedidoEstorno {
  motivo: string;
}

/** A CONFIRMAR: o docs/30 não descreve a forma do estorno. */
export interface Estorno {
  id: string;
  recebimento_id: string;
  situacao: 'pedido' | 'aprovado';
  motivo: string;
  pedido_por: string;
  aprovado_por: string | null;
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

/** A CONFIRMAR: o docs/30 não descreve a forma da contingência. */
export interface Contingencia {
  id: string;
  codigo: string;
  nivel: NivelDeContingencia;
  motivo: string;
  responsavel: string;
  declarada_em: string;
  situacao: 'aberta' | 'encerrada';
}

export interface PedidoFicha {
  formulario: 'F-ATD';
  numero: string;
  paciente_id: string;
  /** Nome como está no papel. */
  nome: string;
  /** Nascimento como está no papel (`AAAA-MM-DD`). */
  nascimento: string;
  ocorrido_em: string;
  atendido_por: string;
  anexo_sha256: string;
}

export interface PedidoRecibo {
  formulario: 'F-REC';
  numero: string;
  paciente_id: string;
  tuss: string;
  valor_papel_centavos: number;
  ocorrido_em: string;
}

/** A CONFIRMAR: forma da divergência aberta (respostas 202 e recibo com preço diferente). */
export interface Divergencia {
  codigo: string;
  tipo: 'identificacao' | 'preco';
  descricao: string;
}

/** Resposta de `POST .../fichas`: 201 com o atendimento ou 202 com a divergência aberta. */
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
 * Códigos de erro estáveis da Fase 1 (docs/30). Outros podem chegar do núcleo.
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
  'triagem-ausente',
  'conduta-medica-pendente',
  'etiqueta-nao-encontrada',
  'guia-em-uso',
  'procedimento-nao-autorizado',
  'senha-vencida',
  'saldo-esgotado',
  'horario-ocupado',
  'sem-vinculo-de-cuidado',
  'laudo-assinado',
  'assinatura-concorrente',
  'laudo-nao-assinado',
  'nao-encontrado',
  'link-expirado',
  'desconto-acima-do-limite',
  'valor-invalido',
  'caixa-fechado',
  'separacao-de-funcoes',
  'ficha-ja-lancada',
  'numero-fora-da-faixa',
  'divergencia-aberta',
  'pendencias',
] as const;

export type CodigoErroFase1 = (typeof CODIGOS_ERRO_FASE1)[number];
