import type { GrandezaDeDose } from '@vidacor/api-client';

export const UNIDADE: Record<GrandezaDeDose, string> = { ctdivol: 'mGy', dlp: 'mGy·cm', dap: 'Gy·cm²' };
export const NOME_DA_GRANDEZA: Record<GrandezaDeDose, string> = { ctdivol: 'CTDIvol', dlp: 'DLP', dap: 'DAP' };

/** Número no formato brasileiro, com até `casas` decimais. */
export function numero(v: number, casas = 1): string {
  return v.toLocaleString('pt-BR', { maximumFractionDigits: casas });
}
