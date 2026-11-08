import type { ItemDaAgendaFase1 } from '@vidacor/api-client';
import { formatarData, formatarHora, type CabecalhoDoMenu } from '@vidacor/design-system';
import { FUSO_DA_CLINICA } from './grade';

/** Cabeçalho do menu do agendamento enquanto o servidor manda as ações (docs/19 § 8). */
export function cabecalhoDoMenu(item: ItemDaAgendaFase1): CabecalhoDoMenu {
  return {
    titulo: item.paciente.nome,
    subtitulo: `Nascimento ${formatarData(item.paciente.nascimento)}, ${formatarHora(item.inicio, FUSO_DA_CLINICA)}, ${item.exame.nome}`,
  };
}
