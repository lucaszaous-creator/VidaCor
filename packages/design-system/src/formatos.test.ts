import { describe, expect, it } from 'vitest';
import { formatarData, formatarDiaPorExtenso, formatarHora } from './formatos';

describe('formatos de leitura', () => {
  it('data de calendário em dd/mm/aaaa', () => {
    expect(formatarData('1954-06-15')).toBe('15/06/1954');
  });

  it('formato desconhecido volta como veio', () => {
    expect(formatarData('15/06/1954')).toBe('15/06/1954');
    expect(formatarHora('ontem')).toBe('ontem');
  });

  it('hora local de um instante', () => {
    expect(formatarHora('2026-11-06T12:00:00Z', 'America/Sao_Paulo')).toBe('09:00');
  });

  it('dia por extenso sem "-feira"', () => {
    expect(formatarDiaPorExtenso('2026-11-06')).toBe('Sexta, 6 de novembro');
    expect(formatarDiaPorExtenso('2026-11-07')).toBe('Sábado, 7 de novembro');
  });
});
