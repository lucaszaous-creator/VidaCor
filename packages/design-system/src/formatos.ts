/**
 * Formatação só para leitura na tela (nenhuma regra de negócio). Datas de calendário chegam do
 * servidor como `AAAA-MM-DD`; instantes como RFC 3339.
 */

const DATA_DE_CALENDARIO = /^(\d{4})-(\d{2})-(\d{2})$/;

/** `1954-06-15` vira `15/06/1954`. Formato desconhecido volta como veio (a tela não inventa). */
export function formatarData(data: string): string {
  const partes = DATA_DE_CALENDARIO.exec(data);
  if (partes === null) {
    return data;
  }
  return `${partes[3] ?? ''}/${partes[2] ?? ''}/${partes[1] ?? ''}`;
}

/** Hora local `HH:MM` de um instante RFC 3339. */
export function formatarHora(instante: string, fusoHorario?: string): string {
  const d = new Date(instante);
  if (Number.isNaN(d.getTime())) {
    return instante;
  }
  return new Intl.DateTimeFormat('pt-BR', {
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
    ...(fusoHorario === undefined ? {} : { timeZone: fusoHorario }),
  }).format(d);
}

/** `2026-11-06` vira "Sexta, 6 de novembro". */
export function formatarDiaPorExtenso(data: string): string {
  const partes = DATA_DE_CALENDARIO.exec(data);
  if (partes === null) {
    return data;
  }
  const d = new Date(Date.UTC(Number(partes[1]), Number(partes[2]) - 1, Number(partes[3]), 12));
  const semana = new Intl.DateTimeFormat('pt-BR', { weekday: 'long', timeZone: 'UTC' }).format(d).replace('-feira', '');
  const resto = new Intl.DateTimeFormat('pt-BR', { day: 'numeric', month: 'long', timeZone: 'UTC' }).format(d);
  return `${semana.charAt(0).toLocaleUpperCase('pt-BR')}${semana.slice(1)}, ${resto}`;
}
