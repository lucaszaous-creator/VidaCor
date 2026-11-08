import { describe, expect, it, vi } from 'vitest';
import { abrirEventos, lerEstudoRecebido, type EstadoDosEventos, type FonteDeEventos } from './eventos';

class FonteFalsa implements FonteDeEventos {
  readyState = 0;
  onopen: FonteDeEventos['onopen'] = null;
  onerror: FonteDeEventos['onerror'] = null;
  fechada = false;
  private readonly ouvintes = new Map<string, ((ev: MessageEvent<string>) => void)[]>();

  addEventListener(tipo: string, ouvinte: (ev: MessageEvent<string>) => void): void {
    this.ouvintes.set(tipo, [...(this.ouvintes.get(tipo) ?? []), ouvinte]);
  }

  close(): void {
    this.fechada = true;
    this.readyState = 2;
  }

  abrir(): void {
    this.readyState = 1;
    this.onopen?.(new Event('open'));
  }

  falhar(fechar: boolean): void {
    this.readyState = fechar ? 2 : 0;
    this.onerror?.(new Event('error'));
  }

  emitir(tipo: string, dados: string): void {
    for (const o of this.ouvintes.get(tipo) ?? []) {
      o(new MessageEvent(tipo, { data: dados }));
    }
  }
}

function montar() {
  const fontes: FonteFalsa[] = [];
  const agendados: { acao: () => void; ms: number; cancelado: boolean }[] = [];
  const estados: EstadoDosEventos[] = [];
  const recebidos: unknown[] = [];
  const descartados: string[] = [];
  const conexao = abrirEventos({
    url: 'http://vidacor.teste/v1/eventos',
    criarFonte: () => {
      const f = new FonteFalsa();
      fontes.push(f);
      return f;
    },
    agendar: (acao, ms) => {
      const item = { acao, ms, cancelado: false };
      agendados.push(item);
      return () => {
        item.cancelado = true;
      };
    },
    aoEstudoRecebido: (e) => recebidos.push(e),
    aoMudarEstado: (e) => estados.push(e),
    aoDescartar: (t) => descartados.push(t),
  });
  return { fontes, agendados, estados, recebidos, descartados, conexao };
}

describe('eventos em tempo real (SSE)', () => {
  it('entrega o evento estudo-recebido conferido', () => {
    const m = montar();
    m.fontes[0]?.abrir();
    m.fontes[0]?.emitir('estudo-recebido', JSON.stringify({ atendimento_id: 'at-1', accession: 'ACC-2026-000001', imagens: 24 }));
    expect(m.recebidos).toEqual([{ atendimento_id: 'at-1', accession: 'ACC-2026-000001', imagens: 24 }]);
    expect(m.estados).toEqual(['conectando', 'conectado']);
  });

  it('descarta evento fora do formato', () => {
    const m = montar();
    m.fontes[0]?.emitir('estudo-recebido', '{"atendimento_id":1}');
    m.fontes[0]?.emitir('estudo-recebido', 'não é json');
    expect(m.recebidos).toEqual([]);
    expect(m.descartados).toEqual(['estudo-recebido', 'estudo-recebido']);
  });

  it('deixa o navegador reconectar enquanto a fonte não fecha', () => {
    const m = montar();
    m.fontes[0]?.falhar(false);
    expect(m.agendados).toHaveLength(0);
    expect(m.estados.at(-1)).toBe('reconectando');
  });

  it('reabre com espera crescente quando a fonte fecha e volta ao normal ao conectar', () => {
    const m = montar();
    m.fontes[0]?.falhar(true);
    expect(m.fontes[0]?.fechada).toBe(true);
    expect(m.agendados.map((a) => a.ms)).toEqual([1000]);
    m.agendados[0]?.acao();
    expect(m.fontes).toHaveLength(2);
    m.fontes[1]?.falhar(true);
    expect(m.agendados.map((a) => a.ms)).toEqual([1000, 2000]);
    m.agendados[1]?.acao();
    m.fontes[2]?.abrir();
    m.fontes[2]?.falhar(true);
    expect(m.agendados.map((a) => a.ms)).toEqual([1000, 2000, 1000]);
  });

  it('fechar cancela a reabertura e fecha a fonte', () => {
    const m = montar();
    m.fontes[0]?.falhar(true);
    m.conexao.fechar();
    expect(m.agendados[0]?.cancelado).toBe(true);
    const criadas = m.fontes.length;
    m.agendados[0]?.acao();
    expect(m.fontes).toHaveLength(criadas);
  });

  it('usa EventSource do navegador quando nenhuma fonte é dada', () => {
    const criado = vi.fn();
    class EventSourceFalso extends FonteFalsa {
      constructor(url: string) {
        super();
        criado(url);
      }
    }
    vi.stubGlobal('EventSource', EventSourceFalso);
    const c = abrirEventos({ url: 'http://vidacor.teste/v1/eventos', aoEstudoRecebido: () => undefined });
    expect(criado).toHaveBeenCalledWith('http://vidacor.teste/v1/eventos');
    c.fechar();
    vi.unstubAllGlobals();
  });
});

describe('lerEstudoRecebido', () => {
  it.each([
    ['{"atendimento_id":"a","accession":"b","imagens":-1}'],
    ['{"atendimento_id":"a","accession":"b","imagens":1.5}'],
    ['{"atendimento_id":"a","imagens":3}'],
    ['null'],
    ['[]'],
  ])('recusa %s', (dados) => {
    expect(lerEstudoRecebido(dados)).toBeNull();
  });

  it('ignora campos extras e devolve só os combinados', () => {
    expect(lerEstudoRecebido('{"atendimento_id":"a","accession":"b","imagens":0,"nome":"x"}')).toEqual({
      atendimento_id: 'a',
      accession: 'b',
      imagens: 0,
    });
  });
});
