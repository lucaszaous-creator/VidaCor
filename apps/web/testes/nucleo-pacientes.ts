/**
 * Lista de pacientes e possíveis duplicados simulados para os testes de TELA. As regras reais (recortes,
 * paginação, decisão única por par, acesso indevido) estão em `crates/api/tests/pacientes_lista.rs`.
 */
import type { DecisaoSobreDuplicado, LinhaDaListaDePacientes, ListaDePacientes, MenuDoObjeto, ParDuplicado } from '@vidacor/api-client';
import { http, HttpResponse } from 'msw';
import { semPermissao, temPerfil } from './nucleo-fase1';
import { problema } from './problema';

export const VERA_ID = '0190a0b0-0000-7000-8000-000000000004';
const MARIA_ID = '0190a0b0-0000-7000-8000-000000000002';
const MARIA_2_ID = '0190a0b0-0000-7000-8000-000000000790';

const linha = (id: string, codigo: string, nome: string, nascimento: string, extra: Partial<LinhaDaListaDePacientes> = {}): LinhaDaListaDePacientes => ({
  id,
  codigo,
  nome,
  nascimento,
  convenio: 'Particular',
  cpf_mascarado: null,
  celular: null,
  alergias: [],
  ultimo_exame: null,
  proximo_exame: null,
  situacao: 'em_dia',
  falta_no_cadastro: [],
  ...extra,
});

const LINHAS: LinhaDaListaDePacientes[] = [
  linha('0190a0b0-0000-7000-8000-000000000001', 'PAC-2026-000001', 'Antônio Carlos Ribeiro', '1954-06-15', {
    convenio: 'Petrobras AMS',
    situacao: 'pendencia',
    proximo_exame: { exame: 'MAPA 24 h', inicio: '2026-11-12T13:40:00Z' },
  }),
  linha(MARIA_ID, 'PAC-2026-000002', 'Maria Helena Duarte', '1968-03-12', { convenio: 'Unimed Pleno', situacao: 'possivel_duplicado', cpf_mascarado: '***.***.***-09', celular: '48998124410' }),
  linha(MARIA_2_ID, 'PAC-2026-000790', 'Maria Helena Duarte', '1968-03-21', { situacao: 'cadastro_incompleto', falta_no_cadastro: ['cpf', 'nome_mae'] }),
  linha('0190a0b0-0000-7000-8000-000000000005', 'PAC-2026-000005', 'Sônia Maria Castro', '1962-02-03', { situacao: 'cadastro_incompleto', falta_no_cadastro: ['cpf', 'celular', 'nome_mae', 'alergias'] }),
  linha(VERA_ID, 'PAC-2026-000004', 'Vera Lúcia Antunes', '1957-09-18', {
    convenio: 'Amil S750',
    situacao: 'na_clinica',
    alergias: ['dipirona'],
    cpf_mascarado: '***.***.***-25',
    proximo_exame: { exame: 'Ecocardiograma transtorácico', inicio: '2026-11-09T17:10:00Z' },
    ultimo_exame: { exame: 'Doppler venoso', inicio: '2026-11-02T12:00:00Z' },
  }),
];

export const pacientesSimulados: { pares: ParDuplicado[]; decisoes: DecisaoSobreDuplicado[] } = { pares: [], decisoes: [] };

export function reiniciarPacientes(): void {
  pacientesSimulados.decisoes = [];
  pacientesSimulados.pares = [
    {
      a: { id: MARIA_ID, codigo: 'PAC-2026-000002', nome: 'Maria Helena Duarte', nascimento: '1968-03-12', cpf_mascarado: '***.***.***-09', celular: '48998124410' },
      b: { id: MARIA_2_ID, codigo: 'PAC-2026-000790', nome: 'Maria Helena Duarte', nascimento: '1968-03-21', cpf_mascarado: null, celular: '48998124410' },
      motivo: 'Mesmo nome e mesmo celular, nascimento diferente',
      situacao: 'aberto',
    },
  ];
}
reiniciarPacientes();

const semAcento = (t: string): string => t.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();
const ve = (): boolean => temPerfil('recepcao', 'gerente', 'tecnico', 'medico');

export const manipuladoresPacientes = [
  http.get('*/v1/pacientes/lista', ({ request }) => {
    if (!ve()) return semPermissao('ver a lista de pacientes');
    const url = new URL(request.url);
    const q = semAcento(url.searchParams.get('q') ?? '');
    const recorte = url.searchParams.get('recorte') ?? 'todos';
    const base = LINHAS.filter((l) => q === '' || semAcento(l.nome).includes(q));
    const doRecorte = (r: string) =>
      base.filter((l) => r === 'todos' || (r === 'hoje' && l.situacao === 'na_clinica') || (r === 'pendencia' && l.situacao === 'pendencia') || (r === 'incompleto' && l.falta_no_cadastro.length > 0));
    const itens = doRecorte(recorte);
    const lista: ListaDePacientes = {
      cadastrados: LINHAS.length,
      recortes: { todos: base.length, hoje: doRecorte('hoje').length, pendencia: doRecorte('pendencia').length, incompleto: doRecorte('incompleto').length, duplicados: pacientesSimulados.pares.length },
      pagina: 1,
      paginas: 1,
      por_pagina: 12,
      total: itens.length,
      itens,
    };
    return HttpResponse.json(lista);
  }),
  http.get('*/v1/pacientes/pares-duplicados', () => (temPerfil('recepcao', 'gerente') ? HttpResponse.json(pacientesSimulados.pares) : semPermissao('ver os possíveis duplicados'))),
  http.post('*/v1/pacientes/pares-duplicados', async ({ request }) => {
    if (!temPerfil('recepcao', 'gerente')) return semPermissao('decidir sobre duplicados');
    const d = (await request.json()) as DecisaoSobreDuplicado;
    const par = pacientesSimulados.pares.find((p) => (p.a.id === d.paciente_a && p.b.id === d.paciente_b) || (p.a.id === d.paciente_b && p.b.id === d.paciente_a));
    if (par === undefined) return problema(404, 'paciente-nao-encontrado', 'Cadastro não encontrado nesta unidade. Nada foi registrado.');
    if (par.situacao === 'juncao_pedida' && (d.motivo ?? '').trim().length < 5) {
      return problema(409, 'par-ja-decidido', 'Este par já tem outra decisão. Para mudar, escreva o motivo; a anterior fica no histórico.');
    }
    pacientesSimulados.decisoes.push(d);
    if (d.decisao === 'pessoas_diferentes') pacientesSimulados.pares = pacientesSimulados.pares.filter((p) => p !== par);
    else par.situacao = 'juncao_pedida';
    return HttpResponse.json({ id: 'dup-1', codigo: `DUP-2026-00000${String(pacientesSimulados.decisoes.length)}`, decisao: d.decisao }, { status: 201 });
  }),
  http.get('*/v1/menu/paciente/:id', ({ params }) => {
    const l = LINHAS.find((x) => x.id === params.id);
    if (l === undefined) return problema(404, 'paciente-nao-encontrado', 'Paciente não encontrado nesta unidade.');
    const menu: MenuDoObjeto = {
      cabecalho: { titulo: l.nome, subtitulo: `${l.nascimento.split('-').reverse().join('/')}, prontuário ${l.codigo}` },
      acoes: [
        { id: 'abrir', rotulo: 'Abrir ficha', estado: 'disponivel', perigo: false, destino: `/pacientes?id=${l.id}` },
        ...(temPerfil('recepcao', 'gerente') ? [{ id: 'agendar', rotulo: 'Agendar exame', estado: 'disponivel' as const, perigo: false, destino: '/agendamentos/novo' }] : []),
        { id: 'copiar', rotulo: 'Copiar nome e nascimento', estado: 'disponivel', perigo: false, destino: null },
      ],
    };
    return HttpResponse.json(menu);
  }),
];
