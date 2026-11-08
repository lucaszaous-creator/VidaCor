/**
 * Núcleo simulado dos cadastros base: corpo clínico, catálogo de exames com tabela de preços e cabeçalho
 * dos documentos. Regras reais em `crates/api/tests/corpo_clinico.rs`, `catalogo.rs` e
 * `documentos_pdf.rs`. Só dados fictícios.
 */
import type {
  CabecalhoDaUnidade,
  CatalogoDeExames,
  MedicoDoCorpoClinico,
  NovaVersaoDaTabela,
  NovoCabecalho,
  NovoPedidoDoCorpoClinico,
  NovoProcedimento,
  MudancaNoProcedimento,
  PedidoDoCorpoClinico,
  ProcedimentoDoCatalogo,
} from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { SESSOES } from './dados/fase1';
import { fase1, reautenticacao, registrar, semPermissao, temPerfil } from './nucleo-fase1';
import { estado } from './nucleo-simulado';
import { problema } from './problema';

const HELENA = '0000000a-0000-7000-8000-000000000099';

function pedidoSimulado(id: string, por: { id: string; nome: string }, registro: PedidoDoCorpoClinico['registro']): PedidoDoCorpoClinico {
  return {
    id,
    codigo: `PCC-2026-0000${id}`,
    tipo: 'registro',
    situacao: 'aguardando',
    registro,
    habilitacao: null,
    motivo: 'cadastro do corpo clínico',
    pedido_por: por.nome,
    pedido_por_id: por.id,
    pedido_em: '2026-11-06T20:20:00Z',
    decidido_por: null,
    decidido_em: null,
    motivo_decisao: null,
  };
}

function medicosIniciais(): MedicoDoCorpoClinico[] {
  return [
    {
      id: '0000000a-0000-7000-8000-000000000004',
      codigo: 'USR-medico.a',
      nome: 'Dr. Paulo Medeiros',
      ativo: true,
      situacao: 'ativo',
      registro: {
        conselho: 'CRM',
        uf: 'SP',
        numero: '123456',
        rqe: '7788',
        especialidade: 'Cardiologia',
        desde: '2026-01-03T12:00:00Z',
      },
      habilitacao: {
        realiza: ['US'],
        lauda: ['US'],
        desde: '2026-01-03T12:00:00Z',
      },
      pendentes: [],
    },
    {
      id: '0000000a-0000-7000-8000-000000000005',
      codigo: 'USR-cecilia.souto',
      nome: 'Dra. Cecília Souto',
      ativo: true,
      situacao: 'aguardando_conferencia',
      registro: null,
      habilitacao: null,
      pendentes: [
        pedidoSimulado(
          '1',
          { id: HELENA, nome: 'Helena Prado' },
          {
            uf: 'SP',
            numero: '654321',
            rqe: null,
            especialidade: 'Radiologia',
          },
        ),
      ],
    },
    {
      id: '0000000a-0000-7000-8000-000000000006',
      codigo: 'USR-marcos.freire',
      nome: 'Dr. Marcos Freire',
      ativo: true,
      situacao: 'sem_registro',
      registro: null,
      habilitacao: null,
      pendentes: [],
    },
  ];
}

function catalogoInicial(): CatalogoDeExames {
  const p = (tuss: string, nome: string, modalidade: string, duracao: number, preco: number | null, extra: Partial<ProcedimentoDoCatalogo> = {}): ProcedimentoDoCatalogo => ({
    tuss,
    nome,
    modalidade,
    duracao_min: duracao,
    requer_contraste: false,
    ativo: true,
    versao: 1,
    preco_centavos: preco,
    proximo_preco_centavos: null,
    ...extra,
  });
  return {
    procedimentos: [
      p('40901114', 'Ecocardiograma transtorácico', 'US', 30, 38000),
      p('40901122', 'USG de tireoide', 'US', 20, 18000),
      p('41001109', 'TC de abdome com contraste', 'CT', 30, 95000, {
        requer_contraste: true,
      }),
      p('40801012', 'Raio X de tórax', 'DX', 15, null, {
        ativo: false,
        versao: 2,
      }),
    ],
    tabelas: [
      {
        id: 't12',
        versao: 12,
        vigente_desde: '2026-01-01',
        descricao: 'Particular 2026',
        motivo: null,
        itens: 3,
        publicada_por: 'Patrícia Lemos',
        publicada_em: '2025-12-20T12:00:00Z',
        situacao: 'vigente',
      },
    ],
  };
}

export const cadastrosSimulados: {
  medicos: MedicoDoCorpoClinico[];
  catalogo: CatalogoDeExames;
  cabecalho: CabecalhoDaUnidade | null;
} = {
  medicos: medicosIniciais(),
  catalogo: catalogoInicial(),
  cabecalho: null,
};

export function reiniciarCadastros(): void {
  cadastrosSimulados.medicos = medicosIniciais();
  cadastrosSimulados.catalogo = catalogoInicial();
  cadastrosSimulados.cabecalho = null;
}

const ehGerente = () => temPerfil('gerente');
const leCatalogo = () => temPerfil('recepcao', 'supervisor_recepcao', 'gerente');

export const manipuladoresCadastros = [
  http.get('*/v1/corpo-clinico', () => (ehGerente() ? HttpResponse.json(cadastrosSimulados.medicos) : semPermissao('administrar o corpo clínico'))),
  http.get('*/v1/corpo-clinico/:id/historico', ({ params }) => {
    if (!ehGerente()) return semPermissao('administrar o corpo clínico');
    const m = cadastrosSimulados.medicos.find((x) => x.id === params.id);
    return m === undefined ? problema(404, 'medico-nao-encontrado', 'Médico não encontrado nesta unidade.') : HttpResponse.json(m.pendentes);
  }),
  http.post('*/v1/corpo-clinico/pedidos/:id/decisao', async ({ request, params }) => {
    const c = (await registrar('decidir-corpo-clinico', request)) as {
      aprovar: boolean;
      motivo: string;
    };
    if (!ehGerente()) return semPermissao('conferir o pedido');
    const m = cadastrosSimulados.medicos.find((x) => x.pendentes.some((p) => p.id === params.id));
    const p = m?.pendentes.find((x) => x.id === params.id);
    if (m === undefined || p === undefined) return problema(404, 'pedido-nao-encontrado', 'Pedido não encontrado nesta unidade. Nada foi alterado.');
    if (p.pedido_por_id === estado.sessao.usuario.id) {
      return problema(403, 'separacao-de-funcoes', 'A conferência é de outra pessoa: quem pediu, e o próprio médico, não decidem. Nada foi alterado.');
    }
    const r = reautenticacao('conferir-corpo-clinico');
    if (r !== null) return r;
    m.pendentes = m.pendentes.filter((x) => x.id !== p.id);
    if (c.aprovar && p.registro !== null && p.registro !== undefined) {
      m.registro = {
        conselho: 'CRM',
        uf: p.registro.uf,
        numero: p.registro.numero,
        rqe: p.registro.rqe ?? null,
        especialidade: p.registro.especialidade ?? null,
        desde: '2026-11-07T12:00:00Z',
      };
    }
    m.situacao = m.pendentes.length > 0 ? 'aguardando_conferencia' : m.registro === null || m.registro === undefined ? 'sem_registro' : 'ativo';
    return HttpResponse.json(m);
  }),
  http.post('*/v1/corpo-clinico/:id/pedidos', async ({ request, params }) => {
    const c = (await registrar('pedir-corpo-clinico', request)) as NovoPedidoDoCorpoClinico;
    if (!ehGerente()) return semPermissao('administrar o corpo clínico');
    const r = reautenticacao('pedir-corpo-clinico');
    if (r !== null) return r;
    const m = cadastrosSimulados.medicos.find((x) => x.id === params.id);
    if (m === undefined) return problema(404, 'medico-nao-encontrado', 'Médico não encontrado nesta unidade. Nada foi alterado.');
    if (c.tipo === 'registro' && c.registro?.numero === '123456' && m.nome !== 'Dr. Paulo Medeiros') {
      return problema(409, 'crm-em-uso', 'Este CRM já está no cadastro de outro médico desta unidade. Nada foi alterado. Confira o número com o documento do conselho.');
    }
    const novo = pedidoSimulado(String(fase1.pedidos.length + 10), { id: SESSOES.gerente.usuario.id, nome: 'Patrícia Lemos' }, c.registro ?? null);
    novo.tipo = c.tipo;
    novo.habilitacao = c.habilitacao ?? null;
    novo.motivo = c.motivo;
    m.pendentes = [...m.pendentes, novo];
    m.situacao = 'aguardando_conferencia';
    return HttpResponse.json(m, { status: 201 });
  }),

  http.get('*/v1/catalogo/procedimentos', () => (leCatalogo() ? HttpResponse.json(cadastrosSimulados.catalogo) : semPermissao('consultar o catálogo'))),
  http.get('*/v1/catalogo/procedimentos/:tuss/historico', ({ params }) =>
    leCatalogo()
      ? HttpResponse.json(
          params.tuss === '40801012'
            ? [
                {
                  codigo: 'PRM-2026-000001',
                  tipo: 'desativar',
                  motivo: 'aparelho em manutenção',
                  versao_do_procedimento: 2,
                  antes: {
                    nome: 'Raio X de tórax',
                    modalidade: 'DX',
                    duracao_min: 15,
                    requer_contraste: false,
                    ativo: true,
                  },
                  depois: {
                    nome: 'Raio X de tórax',
                    modalidade: 'DX',
                    duracao_min: 15,
                    requer_contraste: false,
                    ativo: false,
                  },
                  quem: 'Patrícia Lemos',
                  em: '2026-11-02T12:00:00Z',
                },
              ]
            : [],
        )
      : semPermissao('consultar o catálogo'),
  ),
  http.post('*/v1/catalogo/procedimentos/:tuss/mudancas', async ({ request, params }) => {
    const c = (await registrar('mudar-exame', request)) as MudancaNoProcedimento;
    if (!ehGerente()) return semPermissao('mudar o catálogo');
    const p = cadastrosSimulados.catalogo.procedimentos.find((x) => x.tuss === params.tuss);
    if (p === undefined) return problema(404, 'procedimento-nao-encontrado', 'Procedimento não encontrado no catálogo. Nada foi alterado.');
    if (c.versao !== p.versao)
      return problema(409, 'versao-mudou', 'Outra pessoa mudou este procedimento enquanto você editava. Nada foi alterado. Recarregue e confira antes de repetir.');
    Object.assign(p, {
      duracao_min: c.duracao_min ?? p.duracao_min,
      modalidade: c.modalidade ?? p.modalidade,
      ativo: c.ativo ?? p.ativo,
      versao: p.versao + 1,
    });
    return HttpResponse.json(p);
  }),
  http.post('*/v1/catalogo/procedimentos', async ({ request }) => {
    const c = (await registrar('incluir-exame', request)) as NovoProcedimento;
    if (!ehGerente()) return semPermissao('incluir exame');
    if (cadastrosSimulados.catalogo.procedimentos.some((p) => p.tuss === c.tuss)) {
      return problema(409, 'tuss-em-uso', 'Este código TUSS já está no catálogo. Nada foi incluído. Procure o exame na lista e, se preciso, reative.');
    }
    const p: ProcedimentoDoCatalogo = {
      tuss: c.tuss,
      nome: c.nome,
      modalidade: c.modalidade,
      duracao_min: c.duracao_min,
      requer_contraste: c.requer_contraste,
      ativo: true,
      versao: 1,
      preco_centavos: null,
      proximo_preco_centavos: null,
    };
    cadastrosSimulados.catalogo.procedimentos.push(p);
    return HttpResponse.json(p, { status: 201 });
  }),
  http.post('*/v1/catalogo/tabelas-de-preco', async ({ request }) => {
    const c = (await registrar('publicar-tabela', request)) as NovaVersaoDaTabela;
    if (!ehGerente()) return semPermissao('publicar tabela de preços');
    const t = {
      id: 't13',
      versao: 13,
      vigente_desde: c.vigente_desde,
      descricao: c.descricao,
      motivo: c.motivo,
      itens: c.precos.length,
      publicada_por: 'Patrícia Lemos',
      publicada_em: '2026-11-07T12:00:00Z',
      situacao: 'futura' as const,
    };
    cadastrosSimulados.catalogo.tabelas = [t, ...cadastrosSimulados.catalogo.tabelas];
    return HttpResponse.json(t, { status: 201 });
  }),

  http.get('*/v1/configuracoes/cabecalho', () => {
    if (!ehGerente()) return semPermissao('ver o cabeçalho dos documentos');
    const v = cadastrosSimulados.cabecalho;
    return HttpResponse.json({ vigente: v, versoes: v === null ? [] : [v] });
  }),
  http.post('*/v1/configuracoes/cabecalho', async ({ request }) => {
    const c = (await registrar('publicar-cabecalho', request)) as NovoCabecalho;
    if (!ehGerente()) return semPermissao('publicar o cabeçalho');
    if (c.cnpj.replace(/\D/g, '') !== '11222333000181')
      return problema(422, 'cabecalho-invalido', 'O CNPJ não confere (14 algarismos com os dígitos verificadores). Nada foi publicado.');
    const v: CabecalhoDaUnidade = {
      numero: (c.numero_atual ?? 0) + 1,
      nome: c.nome,
      razao_social: c.razao_social,
      cnpj: '11.222.333/0001-81',
      endereco: c.endereco,
      telefone: c.telefone,
      site: c.site ?? null,
      responsavel_nome: c.responsavel_nome,
      responsavel_crm: `CRM-${c.responsavel_crm_uf} ${c.responsavel_crm_numero}`,
      responsavel_tecnico: `${c.responsavel_nome}, CRM-${c.responsavel_crm_uf} ${c.responsavel_crm_numero}`,
      motivo: c.motivo ?? null,
      publicado_por: 'Patrícia Lemos',
      desde: '2026-11-07T12:00:00Z',
    };
    cadastrosSimulados.cabecalho = v;
    return HttpResponse.json({ vigente: v, versoes: [v] }, { status: 201 });
  }),
];
