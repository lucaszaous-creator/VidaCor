import { describe, expect, it, vi } from 'vitest';
import { criarCliente } from './cliente';
import { ErroDaApi, ehReautenticacao } from './erros';
import type { Sessao } from './contrato-m0';

const BASE = 'http://vidacor.teste';

const SESSAO: Sessao = {
  usuario: { id: 'u-1', nome: 'Bruno Sales', perfis: ['tecnico'] },
  unidade: { id: 'un-1', nome: 'Unidade Centro' },
  csrf: 'csrf-ficticio-2',
};

interface Chamada {
  url: string;
  init: RequestInit;
}

function json(corpo: unknown, status = 200, tipo = 'application/json'): Response {
  return new Response(JSON.stringify(corpo), { status, headers: { 'Content-Type': tipo } });
}

function fetchFalso(respostas: (() => Response)[]) {
  const chamadas: Chamada[] = [];
  const fn = vi.fn((entrada: RequestInfo | URL, init?: RequestInit) => {
    chamadas.push({ url: entrada instanceof Request ? entrada.url : entrada.toString(), init: init ?? {} });
    const proxima = respostas.shift();
    if (proxima === undefined) {
      throw new Error('chamada inesperada');
    }
    return Promise.resolve(proxima());
  });
  return { fn: fn as unknown as typeof fetch, chamadas };
}

function cabecalho(chamada: Chamada | undefined, nome: string): string | undefined {
  return ((chamada?.init.headers ?? {}) as Record<string, string>)[nome];
}

describe('cliente da API, rotas da Fase 1', () => {
  it('manda a estação no cabeçalho X-Estacao quando o formato é válido', async () => {
    const { fn, chamadas } = fetchFalso([() => json([])]);
    const api = criarCliente({ base: BASE, fetch: fn, estacao: 'SALA-1' });
    await api.laudos.fila();
    expect(cabecalho(chamadas[0], 'X-Estacao')).toBe('SALA-1');
  });

  it('não manda estação fora do formato (só letras, números e hífen, até 32)', async () => {
    const { fn, chamadas } = fetchFalso([() => json([])]);
    const api = criarCliente({ base: BASE, fetch: fn, estacao: 'GUICHÊ 1; DROP' });
    await api.laudos.fila();
    expect(cabecalho(chamadas[0], 'X-Estacao')).toBeUndefined();
  });

  it('monta o endereço de reautenticação só com caminho da própria aplicação', () => {
    const api = criarCliente({ base: BASE, fetch: fetchFalso([]).fn });
    expect(api.sessao.urlDeReautenticacao('/laudos/l-1?retomar=assinar')).toBe(
      `${BASE}/v1/sessao/reautenticar?voltar=%2Flaudos%2Fl-1%3Fretomar%3Dassinar`,
    );
    expect(api.sessao.urlDeReautenticacao('https://outro.site/roubo')).toBe(`${BASE}/v1/sessao/reautenticar?voltar=%2F`);
    expect(api.sessao.urlDeReautenticacao('//outro.site')).toBe(`${BASE}/v1/sessao/reautenticar?voltar=%2F`);
  });

  it('bipe manda CSRF e Idempotency-Key e devolve o que o servidor decidiu', async () => {
    const resultado = { worklist: 'enviada', aparelho: { nome: 'Vivid E95', ae_title: 'VIVIDE95' }, atendimento: {} };
    const { fn, chamadas } = fetchFalso([() => json(SESSAO), () => json(resultado)]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await expect(api.salas.bipar('sala 1', { codigo_etiqueta: 'ETQ-2026-018742' }, { idempotencyKey: 'k-1' })).resolves.toEqual(
      resultado,
    );
    expect(chamadas[1]?.url).toBe(`${BASE}/v1/salas/sala%201/bipes`);
    expect(cabecalho(chamadas[1], 'X-CSRF-Token')).toBe('csrf-ficticio-2');
    expect(cabecalho(chamadas[1], 'Idempotency-Key')).toBe('k-1');
  });

  it('rascunho do laudo vai por PUT com CSRF e sem chave de idempotência', async () => {
    const { fn, chamadas } = fetchFalso([() => json(SESSAO), () => json({ id: 'l-1' })]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.laudos.salvarRascunho('l-1', { texto: 'Texto', versao_numero: 1 });
    expect(chamadas[1]?.init.method).toBe('PUT');
    expect(cabecalho(chamadas[1], 'X-CSRF-Token')).toBe('csrf-ficticio-2');
    expect(cabecalho(chamadas[1], 'Idempotency-Key')).toBeUndefined();
    expect(JSON.parse(chamadas[1]?.init.body as string)).toEqual({ texto: 'Texto', versao_numero: 1 });
  });

  it('erro com campos extras (sugestões de horário) fica acessível pelo campo', async () => {
    const corpo = {
      type: 'x',
      title: 'Horário ocupado',
      status: 409,
      detail: 'O horário das 09:00 da Sala 1 acabou de ser ocupado. Nada foi agendado. Escolha um dos próximos horários livres.',
      codigo: 'horario-ocupado',
      trace_id: 't-9',
      sugestoes: ['2026-11-07T09:30:00-03:00'],
    };
    const { fn } = fetchFalso([() => json(SESSAO), () => json(corpo, 409, 'application/problem+json')]);
    const api = criarCliente({ base: BASE, fetch: fn });
    const erro = await api.agendamentos
      .criar(
        { paciente_id: 'p', sala_id: 's', inicio: 'i', procedimentos: ['40901114'], guia_id: null, reserva_provisoria: false },
        { idempotencyKey: 'k' },
      )
      .catch((e: unknown) => e);
    expect(erro).toBeInstanceOf(ErroDaApi);
    expect((erro as ErroDaApi).campo('sugestoes')).toEqual(['2026-11-07T09:30:00-03:00']);
    expect((erro as ErroDaApi).campo('inexistente')).toBeUndefined();
  });

  it('reconhece o pedido de reautenticação', async () => {
    const corpo = { type: 'x', title: 't', status: 401, detail: 'Confirme sua identidade.', codigo: 'reautenticacao-necessaria' };
    const { fn } = fetchFalso([() => json(SESSAO), () => json(corpo, 401, 'application/problem+json')]);
    const api = criarCliente({ base: BASE, fetch: fn });
    const erro = await api.laudos.assinar('l-1', { pin: '1234' }, { idempotencyKey: 'k' }).catch((e: unknown) => e);
    expect(ehReautenticacao(erro)).toBe(true);
    expect(ehReautenticacao(new Error('x'))).toBe(false);
  });

  it('ficha de contingência distingue 201 (lançada) de 202 (divergência aberta)', async () => {
    const divergencia = { codigo: 'DIV-2026-000031', tipo: 'identificacao', descricao: 'Nascimento do papel difere.' };
    const { fn } = fetchFalso([
      () => json(SESSAO),
      () => json({ id: 'a-1', origem: 'contingencia' }, 201),
      () => json({ divergencia }, 202),
    ]);
    const api = criarCliente({ base: BASE, fetch: fn });
    const corpo = {
      formulario: 'F-ATD' as const,
      numero: '0412',
      paciente_id: 'p',
      nome: 'Maria Helena Duarte',
      nascimento: '1968-03-12',
      ocorrido_em: '2026-11-06T10:50:00-03:00',
      atendido_por: 'Dr. Paulo Medeiros',
      anexo_sha256: 'a'.repeat(64),
    };
    await expect(api.contingencias.lancarFicha('c-1', corpo, { idempotencyKey: 'k1' })).resolves.toEqual({
      situacao: 'lancada',
      atendimento: { id: 'a-1', origem: 'contingencia' },
    });
    await expect(api.contingencias.lancarFicha('c-1', corpo, { idempotencyKey: 'k2' })).resolves.toEqual({
      situacao: 'divergencia-aberta',
      divergencia,
    });
  });

  it('declaração de contingência usa a chave da estação como Idempotency-Key', async () => {
    const { fn, chamadas } = fetchFalso([() => json(SESSAO), () => json({ id: 'c-1' }, 201)]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.contingencias.declarar({
      nivel: 'total',
      motivo: 'servidor local fora do ar',
      responsavel_id: 'u-2',
      declarada_em: '2026-11-06T10:42:00-03:00',
      chave: '0190f5e2-0000-7000-8000-000000000001',
    });
    expect(cabecalho(chamadas[1], 'Idempotency-Key')).toBe('0190f5e2-0000-7000-8000-000000000001');
  });

  it('busca de paciente leva o texto só na consulta à API', async () => {
    const { fn, chamadas } = fetchFalso([() => json([])]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.pacientes.buscar('Antônio Carlos');
    expect(chamadas[0]?.url).toBe(`${BASE}/v1/pacientes?busca=Ant%C3%B4nio+Carlos`);
  });
});
