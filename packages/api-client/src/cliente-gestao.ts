/**
 * Rotas da minha agenda do médico, gestão de dose e relatórios (Agente 2). Ficam fora de `cliente.ts`
 * para não crescer o arquivo; `criarCliente` entrega as ferramentas de pedido (sessão, CSRF e
 * `Idempotency-Key` continuam tratados lá).
 */
import type { DocumentoBaixado } from './contrato-lastro';
import type {
  AusenciaDoMedico,
  DecisaoDaAusencia,
  ExamesComDose,
  ExportacaoFeita,
  HistoricoDeDose,
  IdDoRelatorio,
  JustificativaDeDose,
  MinhaAgenda,
  NovaAusencia,
  NovaJustificativa,
  NovoNrd,
  NrdDoExame,
  PeriodoDeConsulta,
  PreviaDoRelatorio,
  ProtocoloDeDose,
  RelatorioDoCatalogo,
  SituacaoDaAusencia,
} from './contrato-gestao';

export interface FerramentasDeGestao {
  ler: <T>(caminho: string) => Promise<T>;
  criar: <T>(metodo: 'POST', caminho: string, corpo: unknown, o: { idempotencyKey: string }) => Promise<T>;
  baixarComEscrita: (caminho: string, corpo: unknown) => Promise<DocumentoBaixado>;
}

function seg(valor: string): string {
  return encodeURIComponent(valor);
}

function consulta(p: PeriodoDeConsulta | undefined): string {
  const q = new URLSearchParams();
  if (p?.de !== undefined && p.de !== '') {
    q.set('de', p.de);
  }
  if (p?.ate !== undefined && p.ate !== '') {
    q.set('ate', p.ate);
  }
  const s = q.toString();
  return s === '' ? '' : `?${s}`;
}

export function rotasDeGestao(f: FerramentasDeGestao) {
  return {
    minhaAgenda: {
      obter: (data?: string): Promise<MinhaAgenda> =>
        f.ler<MinhaAgenda>(`/v1/minha-agenda${data === undefined ? '' : `?${new URLSearchParams({ data }).toString()}`}`),
      /** O médico pede o bloqueio da própria agenda; só vale depois da aprovação da coordenação. */
      pedirAusencia: (corpo: NovaAusencia, o: { idempotencyKey: string }): Promise<AusenciaDoMedico> =>
        f.criar<AusenciaDoMedico>('POST', '/v1/minha-agenda/ausencias', corpo, o),
    },
    ausencias: {
      listar: (situacao?: SituacaoDaAusencia): Promise<AusenciaDoMedico[]> =>
        f.ler<AusenciaDoMedico[]>(`/v1/ausencias${situacao === undefined ? '' : `?${new URLSearchParams({ situacao }).toString()}`}`),
      decidir: (id: string, corpo: DecisaoDaAusencia, o: { idempotencyKey: string }): Promise<AusenciaDoMedico> =>
        f.criar<AusenciaDoMedico>('POST', `/v1/ausencias/${seg(id)}/decisao`, corpo, o),
      cancelar: (id: string, o: { idempotencyKey: string }): Promise<AusenciaDoMedico> =>
        f.criar<AusenciaDoMedico>('POST', `/v1/ausencias/${seg(id)}/cancelamento`, {}, o),
    },
    doses: {
      listar: (p?: PeriodoDeConsulta): Promise<ExamesComDose> => f.ler<ExamesComDose>(`/v1/doses${consulta(p)}`),
      protocolos: (p?: PeriodoDeConsulta): Promise<ProtocoloDeDose[]> =>
        f.ler<ProtocoloDeDose[]>(`/v1/doses/protocolos${consulta(p)}`),
      historico: (pacienteId: string): Promise<HistoricoDeDose> =>
        f.ler<HistoricoDeDose>(`/v1/doses/pacientes/${seg(pacienteId)}`),
      justificar: (doseId: string, corpo: NovaJustificativa, o: { idempotencyKey: string }): Promise<JustificativaDeDose> =>
        f.criar<JustificativaDeDose>('POST', `/v1/doses/${seg(doseId)}/justificativa`, corpo, o),
      nrd: (): Promise<NrdDoExame[]> => f.ler<NrdDoExame[]>('/v1/nrd'),
      publicarNrd: (tuss: string, corpo: NovoNrd, o: { idempotencyKey: string }): Promise<NrdDoExame> =>
        f.criar<NrdDoExame>('POST', `/v1/nrd/${seg(tuss)}`, corpo, o),
    },
    relatorios: {
      catalogo: (): Promise<RelatorioDoCatalogo[]> => f.ler<RelatorioDoCatalogo[]>('/v1/relatorios'),
      previa: (id: IdDoRelatorio, p?: PeriodoDeConsulta): Promise<PreviaDoRelatorio> =>
        f.ler<PreviaDoRelatorio>(`/v1/relatorios/${seg(id)}${consulta(p)}`),
      /** CSV com reautenticação; o servidor registra o SHA-256 e nada fica guardado no navegador. */
      exportar: (id: IdDoRelatorio, p: PeriodoDeConsulta): Promise<DocumentoBaixado> =>
        f.baixarComEscrita(`/v1/relatorios/${seg(id)}/exportacao`, p),
      exportacoes: (): Promise<ExportacaoFeita[]> => f.ler<ExportacaoFeita[]>('/v1/relatorios/exportacoes'),
    },
  };
}
