import type { VersaoDoPacoteDeCheckup } from '@vidacor/api-client';

export const CHAVE_PACOTES = ['pacotes-de-checkup'];

export const SITUACAO_DA_VERSAO: Record<VersaoDoPacoteDeCheckup['situacao'], string> = {
  vigente: 'vigente',
  futura: 'começa no futuro',
  anterior: 'anterior',
};

/** "2027-01-01" vira "01/01/2027" (data de calendário, sem fuso). */
export function dataCurta(iso: string): string {
  const [a = '', m = '', d = ''] = iso.split('-');
  return `${d}/${m}/${a}`;
}

/** Versão que a tela mostra primeiro: a vigente; sem ela, a mais nova. */
export function versaoPrincipal(versoes: VersaoDoPacoteDeCheckup[]): VersaoDoPacoteDeCheckup | undefined {
  return versoes.find((v) => v.situacao === 'vigente') ?? versoes[0];
}
