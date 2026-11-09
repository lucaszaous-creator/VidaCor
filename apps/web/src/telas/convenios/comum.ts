import type { CorDoChip } from '@vidacor/design-system';
import type { RegraDaAuditoriaDoConvenio, SituacaoDoConvenio, VersaoDaTabelaDoConvenio } from '@vidacor/api-client';

export const CHAVE_CONVENIOS = ['convenios'];

export const SITUACAO: Record<SituacaoDoConvenio, { texto: string; cor: CorDoChip }> = {
  vigente: { texto: 'Vigente', cor: 'verde' },
  renovar: { texto: 'Renovar', cor: 'ambar' },
  vencido: { texto: 'Vencido', cor: 'rosa' },
  sem_vigencia: { texto: 'Sem vigência', cor: 'cinza' },
  inativo: { texto: 'Inativo', cor: 'cinza' },
};

export const SITUACAO_DA_TABELA: Record<VersaoDaTabelaDoConvenio['situacao'], string> = {
  vigente: 'vigente',
  futura: 'começa no futuro',
  anterior: 'anterior',
};

export const EFEITO: Record<RegraDaAuditoriaDoConvenio['efeito'], { texto: string; cor: CorDoChip }> = {
  bloqueia: { texto: 'Bloqueia', cor: 'rosa' },
  corrigir_antes: { texto: 'Corrigir antes', cor: 'ambar' },
  atencao: { texto: 'Atenção', cor: 'azul' },
};

/** "2027-01-01" vira "01/01/2027" (data de calendário, sem fuso). */
export function dataCurta(iso: string): string {
  const [a = '', m = '', d = ''] = iso.split('-');
  return `${d}/${m}/${a}`;
}
