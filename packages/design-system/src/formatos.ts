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

/**
 * Centavos (inteiro, como o servidor manda) em reais para leitura: `38000` vira `R$ 380,00`.
 * Só formata; nenhuma conta de negócio é feita aqui. Valor que não é inteiro volta como veio.
 */
export function formatarDinheiro(centavos: number): string {
  if (!Number.isInteger(centavos)) {
    return String(centavos);
  }
  const reais = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' }).format(centavos / 100);
  return reais.replace(/\u00a0/g, ' ');
}

/** Pontos base em percentual para leitura: `500` vira `5,00%`. */
export function formatarPontosBase(pontosBase: number): string {
  if (!Number.isInteger(pontosBase)) {
    return String(pontosBase);
  }
  return `${new Intl.NumberFormat('pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(pontosBase / 100)}%`;
}

/** Data e hora local de um instante RFC 3339: `06/11/2026 09:00`. */
export function formatarDataHora(instante: string, fusoHorario?: string): string {
  const d = new Date(instante);
  if (Number.isNaN(d.getTime())) {
    return instante;
  }
  const opcoes = fusoHorario === undefined ? {} : { timeZone: fusoHorario };
  const data = new Intl.DateTimeFormat('pt-BR', { day: '2-digit', month: '2-digit', year: 'numeric', ...opcoes }).format(d);
  return `${data} ${formatarHora(instante, fusoHorario)}`;
}

/**
 * Lê um valor em reais digitado (`380`, `380,5`, `1.380,50`) e devolve centavos, ou `null` se não
 * for um valor no formato brasileiro. É conversão de formato do que a pessoa digitou (como a data),
 * não cálculo: o servidor confere e decide.
 */
export function lerReais(texto: string): number | null {
  const limpo = texto.trim().replace(/^R\$\s*/, '');
  if (!/^\d{1,3}(\.\d{3})*(,\d{1,2})?$|^\d+(,\d{1,2})?$/.test(limpo)) {
    return null;
  }
  const [inteiro = '0', fracao = ''] = limpo.replace(/\./g, '').split(',');
  return Number(inteiro) * 100 + Number(fracao.padEnd(2, '0'));
}

/** Lê um percentual digitado (`15`, `5,5`) e devolve pontos base (`1500`, `550`), ou `null`. */
export function lerPercentual(texto: string): number | null {
  const limpo = texto.trim().replace(/%$/, '').trim();
  if (!/^\d{1,3}(,\d{1,2})?$/.test(limpo)) {
    return null;
  }
  const [inteiro = '0', fracao = ''] = limpo.split(',');
  return Number(inteiro) * 100 + Number(fracao.padEnd(2, '0'));
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
