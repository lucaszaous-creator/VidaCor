/** TV de chamada (Telao.dc.html; docs/18: a TV vê só senha, nome abreviado e destino). Conferido em `conformidade.ts`. */

export interface PedidoDeChamada {
  /** `guiche` (recepção) ou `sala` (técnico lotado na sala do atendimento). */
  destino: 'guiche' | 'sala';
  guiche?: number | null;
}

export interface ChamadaNoPainel {
  senha: string;
  nome_abreviado: string;
  destino: string;
  rechamada: boolean;
  chamada_em: string;
}

export interface Painel {
  agora?: ChamadaNoPainel | null;
  ultimas: ChamadaNoPainel[];
}
