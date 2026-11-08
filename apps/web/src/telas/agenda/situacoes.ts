import type { CorDoChip } from '@vidacor/design-system';

/**
 * Como cada situação vinda do servidor aparece (cor MAIS texto). Só apresentação: quem muda a
 * situação é o servidor. Valor desconhecido não vira outra coisa: aparece como "não reconhecida".
 */
interface Aparencia {
  texto: string;
  cor: CorDoChip;
  /** Classe do cartão na grade (Agenda.dc.html: `done`, `now`, `wait`, `cx`). */
  classe: string;
  /** Cor do ponto no cartão, quando há (Agenda.dc.html). */
  ponto?: string;
}

const AGENDAMENTO: Record<string, Aparencia> = {
  agendado: { texto: 'Agendado', cor: 'azul', classe: '' },
  reserva_provisoria: { texto: 'Reserva provisória', cor: 'ambar', classe: 'wait' },
  expirada: { texto: 'Reserva expirada', cor: 'cinza', classe: 'cx' },
  confirmado: { texto: 'Confirmado', cor: 'verdeAgua', classe: '' },
  em_atendimento: { texto: 'Em atendimento', cor: 'violeta', classe: 'now', ponto: '#5B45B5' },
  concluido: { texto: 'Concluído', cor: 'cinza', classe: 'done' },
  faltou: { texto: 'Faltou', cor: 'rosa', classe: 'done' },
  cancelado: { texto: 'Cancelado', cor: 'cinza', classe: 'cx' },
};

const ATENDIMENTO: Record<string, Aparencia> = {
  aguardando_sala: { texto: 'Aguardando sala', cor: 'ambar', classe: 'wait' },
  aguardando_exame: { texto: 'Aguardando exame', cor: 'ambar', classe: 'wait' },
  em_exame: { texto: 'Em exame', cor: 'violeta', classe: 'now' },
  exame_concluido: { texto: 'Exame concluído', cor: 'cinza', classe: 'done' },
  cancelado: { texto: 'Cancelado', cor: 'cinza', classe: 'cx' },
};

const DESCONHECIDA: Aparencia = { texto: 'Situação não reconhecida', cor: 'cinza', classe: '' };

export function aparenciaDoAgendamento(situacao: string): Aparencia {
  return AGENDAMENTO[situacao] ?? DESCONHECIDA;
}

export function aparenciaDoAtendimento(situacao: string): Aparencia {
  return ATENDIMENTO[situacao] ?? DESCONHECIDA;
}
