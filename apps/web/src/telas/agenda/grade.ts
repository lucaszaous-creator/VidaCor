import type { ItemDaAgenda, Sala } from '@vidacor/api-client';

/** A agenda é do horário de Brasília (openapi.json, parâmetro `data`), seja qual for o fuso do computador. */
export const FUSO_DA_CLINICA = 'America/Sao_Paulo';

const MINUTOS_POR_LINHA = 30;
const INICIO_PADRAO = 7 * 60;
const FIM_PADRAO = 19 * 60;

const formatoHoraMinuto = new Intl.DateTimeFormat('pt-BR', {
  hour: '2-digit',
  minute: '2-digit',
  hourCycle: 'h23',
  timeZone: FUSO_DA_CLINICA,
});

/** Minutos desde a meia-noite, no fuso da clínica. `null` se o instante não é válido. */
export function minutosDoDia(instante: string): number | null {
  const d = new Date(instante);
  if (Number.isNaN(d.getTime())) {
    return null;
  }
  const partes = formatoHoraMinuto.formatToParts(d);
  const hora = Number(partes.find((p) => p.type === 'hour')?.value);
  const minuto = Number(partes.find((p) => p.type === 'minute')?.value);
  return hora * 60 + minuto;
}

export function rotuloDaHora(minutos: number): string {
  return `${String(Math.floor(minutos / 60)).padStart(2, '0')}:${String(minutos % 60).padStart(2, '0')}`;
}

export interface ItemPosicionado {
  item: ItemDaAgenda;
  coluna: number;
  linha: number;
  extensao: number;
}

export interface Grade {
  colunas: Sala[];
  inicio: number;
  linhas: number;
  itens: ItemPosicionado[];
  /** Itens com horário que a tela não conseguiu ler: listados à parte, nunca escondidos. */
  semHorario: ItemDaAgenda[];
}

/**
 * Posição de cada agendamento na grade (sala por coluna, meia hora por linha). Só desenho: a ordem,
 * os horários e as salas vêm do servidor.
 */
export function montarGrade(itens: ItemDaAgenda[], salas: Sala[], salaFiltrada?: string): Grade {
  const colunas: Sala[] = salas.filter((s) => salaFiltrada === undefined || s.id === salaFiltrada);
  for (const item of itens) {
    if (!colunas.some((s) => s.id === item.sala.id) && (salaFiltrada === undefined || item.sala.id === salaFiltrada)) {
      colunas.push(item.sala);
    }
  }
  const comHorario: { item: ItemDaAgenda; ini: number; fim: number }[] = [];
  const semHorario: ItemDaAgenda[] = [];
  for (const item of itens) {
    const ini = minutosDoDia(item.inicio);
    const fim = minutosDoDia(item.fim);
    if (ini === null || fim === null) {
      semHorario.push(item);
    } else {
      comHorario.push({ item, ini, fim: Math.max(fim, ini + 1) });
    }
  }
  const menor = Math.min(INICIO_PADRAO, ...comHorario.map((c) => c.ini));
  const maior = Math.max(FIM_PADRAO, ...comHorario.map((c) => c.fim));
  const inicio = Math.floor(menor / 60) * 60;
  const fim = Math.ceil(maior / 60) * 60;
  const posicionados = comHorario
    .filter((c) => colunas.some((s) => s.id === c.item.sala.id))
    .map((c) => ({
      item: c.item,
      coluna: colunas.findIndex((s) => s.id === c.item.sala.id) + 2,
      linha: Math.floor((c.ini - inicio) / MINUTOS_POR_LINHA) + 2,
      extensao: Math.max(1, Math.ceil((c.fim - c.ini) / MINUTOS_POR_LINHA)),
    }));
  return { colunas, inicio, linhas: (fim - inicio) / MINUTOS_POR_LINHA, itens: posicionados, semHorario };
}

/** "Antônio Carlos Ribeiro" vira "Antônio C. Ribeiro" no cartão da grade (Agenda.dc.html). */
export function nomeCurto(nome: string): string {
  const p = nome.trim().split(/\s+/);
  if (p.length <= 2) {
    return nome;
  }
  return `${p[0] ?? ''} ${(p[1] ?? '').charAt(0)}. ${p[p.length - 1] ?? ''}`;
}

/** Hoje no fuso da clínica, `AAAA-MM-DD`. */
export function hojeNaClinica(agora: Date = new Date()): string {
  return new Intl.DateTimeFormat('en-CA', { timeZone: FUSO_DA_CLINICA, year: 'numeric', month: '2-digit', day: '2-digit' }).format(agora);
}

/** Soma dias a uma data de calendário `AAAA-MM-DD`. */
export function somarDias(data: string, dias: number): string {
  const [a, m, d] = data.split('-').map(Number);
  const base = new Date(Date.UTC(a ?? 1970, (m ?? 1) - 1, d ?? 1));
  base.setUTCDate(base.getUTCDate() + dias);
  return base.toISOString().slice(0, 10);
}
