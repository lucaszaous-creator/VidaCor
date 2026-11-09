import type { MedicoDoCorpoClinico } from '@vidacor/api-client';

export const CHAVE_CORPO_CLINICO = ['corpo-clinico'];

export const SITUACAO_DO_MEDICO: Record<MedicoDoCorpoClinico['situacao'], { texto: string; cor: 'verde' | 'ambar' | 'cinza' | 'rosa' }> = {
  ativo: { texto: 'Ativo', cor: 'verde' },
  aguardando_conferencia: { texto: 'Aguarda conferência', cor: 'ambar' },
  sem_registro: { texto: 'Sem registro', cor: 'rosa' },
  inativo: { texto: 'Inativo', cor: 'cinza' },
};

/** "CRM-SP 123456, Cardiologia" ou o aviso de que falta o registro. */
export function resumoDoRegistro(m: MedicoDoCorpoClinico): string {
  if (m.registro === null || m.registro === undefined) {
    return 'Registro profissional não cadastrado';
  }
  const r = m.registro;
  const esp = typeof r.especialidade === 'string' ? `, ${r.especialidade}` : '';
  return `${r.conselho}-${r.uf} ${r.numero}${esp}`;
}
