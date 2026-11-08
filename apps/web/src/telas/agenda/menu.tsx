import type { ItemDaAgenda } from '@vidacor/api-client';
import { formatarData, formatarHora, type AcaoDoMenu, type CabecalhoDoMenu } from '@vidacor/design-system';
import { FUSO_DA_CLINICA } from './grade';

/**
 * Ações do menu de contexto do agendamento. Vêm do servidor (docs/19 § 8); o contrato v1 do M0 ainda
 * não tem a rota de ações, então a lista é vazia e o menu diz o motivo. Nada é inventado na tela.
 */
export const ACOES_DO_AGENDAMENTO: AcaoDoMenu[] = [];

export const MOTIVO_SEM_ACOES = 'Disponível na próxima entrega';

export function cabecalhoDoMenu(item: ItemDaAgenda): CabecalhoDoMenu {
  return {
    titulo: item.paciente.nome,
    subtitulo: `Nascimento ${formatarData(item.paciente.nascimento)}, ${formatarHora(item.inicio, FUSO_DA_CLINICA)}, ${item.exame.nome}`,
  };
}
