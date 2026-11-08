import { describe, expect, it, vi } from 'vitest';
import { criarCliente, novaChaveDeIdempotencia } from './cliente';
import { ErroDaApi, ErroSemConexao } from './erros';
import type { Sessao } from './contrato-m0';

const BASE = 'http://vidacor.teste';

const SESSAO: Sessao = {
  usuario: { id: 'u-1', nome: 'Ana Souza', perfis: ['recepcao'] },
  unidade: { id: 'un-1', nome: 'Unidade Centro' },
  csrf: 'csrf-ficticio-1',
};

function json(corpo: unknown, status = 200, tipo = 'application/json'): Response {
  return new Response(JSON.stringify(corpo), { status, headers: { 'Content-Type': tipo } });
}

interface Chamada { url: string; init: RequestInit }

function fetchFalso(respostas: (() => Response | Promise<Response>)[]) {
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
  const h = (chamada?.init.headers ?? {}) as Record<string, string>;
  return h[nome];
}

const CORPO_CHECKIN = {
  agendamento_id: 'ag-1',
  confirmacao: { nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' },
};

const ATENDIMENTO = {
  id: 'at-1',
  codigo: 'ATD-2026-000001',
  accession: 'ACC-2026-000001',
  agendamento_id: 'ag-1',
  paciente: { id: 'p-1', nome: 'Antônio Carlos Ribeiro', nascimento: '1954-06-15' },
  situacao: 'aguardando_sala',
  criado_em: '2026-11-06T12:01:00Z',
};

const semEspera = (): Promise<void> => Promise.resolve();

describe('cliente da API', () => {
  it('pede a sessão com cookie e sem guardar nada no navegador', async () => {
    const { fn, chamadas } = fetchFalso([() => json(SESSAO)]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await expect(api.sessao.obter()).resolves.toEqual(SESSAO);
    expect(chamadas[0]?.url).toBe(`${BASE}/v1/sessao`);
    expect(chamadas[0]?.init.credentials).toBe('include');
    expect(chamadas[0]?.init.cache).toBe('no-store');
  });

  it('manda CSRF e Idempotency-Key no check-in', async () => {
    const { fn, chamadas } = fetchFalso([() => json(SESSAO), () => json(ATENDIMENTO, 201)]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.sessao.obter();
    const chave = novaChaveDeIdempotencia();
    await expect(api.atendimentos.criar(CORPO_CHECKIN, { idempotencyKey: chave })).resolves.toEqual(ATENDIMENTO);
    const post = chamadas[1];
    expect(post?.init.method).toBe('POST');
    expect(post?.init.credentials).toBe('include');
    expect(cabecalho(post, 'X-CSRF-Token')).toBe('csrf-ficticio-1');
    expect(cabecalho(post, 'Idempotency-Key')).toBe(chave);
    expect(cabecalho(post, 'Content-Type')).toBe('application/json');
    expect(JSON.parse(post?.init.body as string)).toEqual(CORPO_CHECKIN);
  });

  it('busca a sessão antes da primeira escrita para obter o CSRF', async () => {
    const { fn, chamadas } = fetchFalso([() => json(SESSAO), () => json(ATENDIMENTO, 201)]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.atendimentos.criar(CORPO_CHECKIN, { idempotencyKey: novaChaveDeIdempotencia() });
    expect(chamadas.map((c) => c.url)).toEqual([`${BASE}/v1/sessao`, `${BASE}/v1/atendimentos`]);
    expect(cabecalho(chamadas[1], 'X-CSRF-Token')).toBe('csrf-ficticio-1');
  });

  it('reenvia a escrita com a MESMA chave quando a rede falha', async () => {
    const quedaDeRede = (): Response => {
      throw new TypeError('Failed to fetch');
    };
    const { fn, chamadas } = fetchFalso([
      () => json(SESSAO),
      quedaDeRede,
      quedaDeRede,
      () => json(ATENDIMENTO, 201),
    ]);
    const esperas: number[] = [];
    const api = criarCliente({
      base: BASE,
      fetch: fn,
      esperar: (ms) => {
        esperas.push(ms);
        return Promise.resolve();
      },
    });
    const chave = novaChaveDeIdempotencia();
    await expect(api.atendimentos.criar(CORPO_CHECKIN, { idempotencyKey: chave })).resolves.toEqual(ATENDIMENTO);
    const posts = chamadas.filter((c) => c.url.endsWith('/v1/atendimentos'));
    expect(posts).toHaveLength(3);
    expect(posts.map((c) => cabecalho(c, 'Idempotency-Key'))).toEqual([chave, chave, chave]);
    expect(esperas).toEqual([500, 1000]);
  });

  it('desiste depois das retentativas e avisa que não houve conexão', async () => {
    const quedaDeRede = (): Response => {
      throw new TypeError('Failed to fetch');
    };
    const { fn } = fetchFalso([() => json(SESSAO), quedaDeRede, quedaDeRede, quedaDeRede]);
    const api = criarCliente({ base: BASE, fetch: fn, esperar: semEspera });
    await expect(
      api.atendimentos.criar(CORPO_CHECKIN, { idempotencyKey: novaChaveDeIdempotencia() }),
    ).rejects.toBeInstanceOf(ErroSemConexao);
  });

  it('não reenvia leitura sozinho (o TanStack Query decide)', async () => {
    const { fn, chamadas } = fetchFalso([
      () => {
        throw new TypeError('Failed to fetch');
      },
    ]);
    const api = criarCliente({ base: BASE, fetch: fn, esperar: semEspera });
    await expect(api.salas.listar()).rejects.toBeInstanceOf(ErroSemConexao);
    expect(chamadas).toHaveLength(1);
  });

  it('transforma problem+json em erro tipado, sem reenviar', async () => {
    const problema = {
      type: 'https://vidacor.local/erros/identificacao-nao-confere',
      title: 'Identificação não confere',
      status: 422,
      detail: 'O nascimento informado não confere com o cadastro. Nada foi gravado. Confira o documento.',
      codigo: 'identificacao-nao-confere',
      trace_id: '4bf92f3577b34da6a3ce929d0e0e4736',
    };
    const { fn, chamadas } = fetchFalso([
      () => json(SESSAO),
      () => json(problema, 422, 'application/problem+json'),
    ]);
    const api = criarCliente({ base: BASE, fetch: fn, esperar: semEspera });
    const erro = await api.atendimentos
      .criar(CORPO_CHECKIN, { idempotencyKey: novaChaveDeIdempotencia() })
      .catch((e: unknown) => e);
    expect(erro).toBeInstanceOf(ErroDaApi);
    const e = erro as ErroDaApi;
    expect(e.status).toBe(422);
    expect(e.codigo).toBe('identificacao-nao-confere');
    expect(e.message).toBe(problema.detail);
    expect(e.traceId).toBe(problema.trace_id);
    expect(chamadas).toHaveLength(2);
  });

  it('não inventa mensagem quando a resposta de erro vem fora do formato', async () => {
    const { fn } = fetchFalso([() => new Response('<html>Bad Gateway</html>', { status: 502 })]);
    const api = criarCliente({ base: BASE, fetch: fn });
    const erro = (await api.salas.listar().catch((e: unknown) => e)) as ErroDaApi;
    expect(erro).toBeInstanceOf(ErroDaApi);
    expect(erro.codigo).toBe('resposta-inesperada');
    expect(erro.status).toBe(502);
    expect(erro.message).toContain('Nada foi confirmado');
  });

  it('monta a consulta da agenda com data e sala', async () => {
    const { fn, chamadas } = fetchFalso([() => json({ data: '2026-11-06', itens: [] }), () => json({ data: '2026-11-06', itens: [] })]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.agenda.obter({ data: '2026-11-06' });
    await api.agenda.obter({ data: '2026-11-06', sala_id: 'sala 1' });
    expect(chamadas[0]?.url).toBe(`${BASE}/v1/agenda?data=2026-11-06`);
    expect(chamadas[1]?.url).toBe(`${BASE}/v1/agenda?data=2026-11-06&sala_id=sala+1`);
  });

  it('sair manda CSRF e esquece o token; a escrita seguinte pede a sessão de novo', async () => {
    const { fn, chamadas } = fetchFalso([
      () => json(SESSAO),
      () => new Response(null, { status: 204 }),
      () => json({ ...SESSAO, csrf: 'csrf-ficticio-2' }),
      () => json(ATENDIMENTO, 201),
    ]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.sessao.obter();
    await api.sessao.sair();
    expect(cabecalho(chamadas[1], 'X-CSRF-Token')).toBe('csrf-ficticio-1');
    await api.atendimentos.criar(CORPO_CHECKIN, { idempotencyKey: novaChaveDeIdempotencia() });
    expect(chamadas[2]?.url).toBe(`${BASE}/v1/sessao`);
    expect(cabecalho(chamadas[3], 'X-CSRF-Token')).toBe('csrf-ficticio-2');
  });

  it('codifica o id do atendimento no caminho', async () => {
    const { fn, chamadas } = fetchFalso([() => json({ ...ATENDIMENTO, estudo: null })]);
    const api = criarCliente({ base: BASE, fetch: fn });
    await api.atendimentos.obter('a/b');
    expect(chamadas[0]?.url).toBe(`${BASE}/v1/atendimentos/a%2Fb`);
  });

  it('gera chaves de idempotência UUID distintas', () => {
    const a = novaChaveDeIdempotencia();
    const b = novaChaveDeIdempotencia();
    expect(a).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    expect(a).not.toBe(b);
  });

  it('dá a URL de entrada e a de eventos na origem configurada', () => {
    const api = criarCliente({ base: BASE, fetch: fetchFalso([]).fn });
    expect(api.sessao.urlDeEntrada()).toBe(`${BASE}/v1/sessao/entrar`);
    expect(api.urlDeEventos()).toBe(`${BASE}/v1/eventos`);
  });
});
