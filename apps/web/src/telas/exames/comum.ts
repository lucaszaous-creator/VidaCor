export const CHAVE_CATALOGO = ['catalogo-de-exames'];

/** "2027-01-01" vira "01/01/2027" (data de calendário, sem fuso). */
export function dataCurta(iso: string): string {
  const [a = '', m = '', d = ''] = iso.split('-');
  return `${d}/${m}/${a}`;
}

/** Centavos para o campo de digitação ("380,00"). */
export function reaisParaCampo(centavos: number): string {
  return `${String(Math.floor(centavos / 100))},${String(centavos % 100).padStart(2, '0')}`;
}
