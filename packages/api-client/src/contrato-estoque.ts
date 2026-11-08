/** Estoque com lote e contraste (migration 0022). Conferido em `conformidade.ts`. */
import type { PacienteResumo } from './contrato-m0';

export interface LoteDoItem {
  id: string;
  numero: string;
  validade: string;
  saldo: number;
  vencido: boolean;
}

export interface ItemDeEstoque {
  id: string;
  codigo: string;
  nome: string;
  medida: string;
  minimo: number;
  local: string;
  contraste: boolean;
  saldo_valido: number;
  abaixo_do_minimo: boolean;
  /** Na ordem de saída: vence antes, sai antes. */
  lotes: LoteDoItem[];
}

export interface EntradaDeEstoque {
  item_id: string;
  numero_lote: string;
  validade: string;
  quantidade: number;
}

export interface ContrasteAplicado {
  item_id: string;
  quantidade: number;
}

export interface MovimentoFeito {
  movimento_id: string;
  lote_id: string;
  numero_lote: string;
  validade: string;
  saldo_do_lote: number;
}

export interface PacienteDoLote {
  paciente: PacienteResumo;
  atendimento_codigo: string;
  quantidade: number;
  aplicado_em: string;
}

export interface RastreioDoLote {
  numero: string;
  item: string;
  validade: string;
  pacientes: PacienteDoLote[];
}
