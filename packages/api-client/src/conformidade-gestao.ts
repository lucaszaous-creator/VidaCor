/**
 * Conferência em tempo de compilação dos tipos de `contrato-gestao.ts` com os gerados do núcleo
 * (mesma regra de `conformidade.ts`: se o núcleo mudar um campo, o `typecheck` quebra aqui).
 */
import type { components } from './gerado/openapi';
import type {
  AusenciaDoMedico,
  DecisaoDaAusencia,
  ExameComDose,
  ExamesComDose,
  ExportacaoFeita,
  HistoricoDeDose,
  ItemDaMinhaAgenda,
  MinhaAgenda,
  NovaAusencia,
  NovaJustificativa,
  NovoNrd,
  NrdDoExame,
  PreviaDoRelatorio,
  ProtocoloDeDose,
  RelatorioDoCatalogo,
} from './contrato-gestao';

type S = components['schemas'];
type Cabe<A extends B, B> = A;

export type ConfereItemDaMinhaAgenda = Cabe<ItemDaMinhaAgenda, S['ItemDaMinhaAgenda']>;
export type ConfereMinhaAgenda = Cabe<MinhaAgenda, S['MinhaAgenda']>;
export type ConfereAusencia = Cabe<AusenciaDoMedico, S['AusenciaDoMedico']>;
export type ConfereNovaAusencia = Cabe<NovaAusencia, S['NovaAusencia']>;
export type ConfereDecisaoDaAusencia = Cabe<DecisaoDaAusencia, S['DecisaoDaAusencia']>;
export type ConfereExameComDose = Cabe<ExameComDose, S['ExameComDose']>;
export type ConfereExamesComDose = Cabe<ExamesComDose, S['ExamesComDose']>;
export type ConfereHistoricoDeDose = Cabe<HistoricoDeDose, S['HistoricoDeDose']>;
export type ConfereProtocoloDeDose = Cabe<ProtocoloDeDose, S['ProtocoloDeDose']>;
export type ConfereNovaJustificativa = Cabe<NovaJustificativa, S['NovaJustificativa']>;
export type ConfereNrdDoExame = Cabe<NrdDoExame, S['NrdDoExame']>;
export type ConfereNovoNrd = Cabe<NovoNrd, S['NovoNrd']>;
export type ConfereRelatorioDoCatalogo = Cabe<RelatorioDoCatalogo, S['RelatorioDoCatalogo']>;
export type ConferePreviaDoRelatorio = Cabe<PreviaDoRelatorio, S['PreviaDoRelatorio']>;
export type ConfereExportacaoFeita = Cabe<ExportacaoFeita, S['ExportacaoFeita']>;
