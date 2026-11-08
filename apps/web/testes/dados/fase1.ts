/**
 * Dados FICTÍCIOS da Fase 1, tirados de design/dados-ficticios.md e das frases do docs/27.
 * Nenhum dado real de paciente. Só sintaxe apagável de TypeScript: roda também direto no Node
 * (verificação "a tela cabe na janela").
 */
import type {
  AtendimentoDetalhadoFase1,
  Caixa,
  Contingencia,
  FilaDaSala,
  HorarioLivre,
  ItemDaFilaDeLaudos,
  ItemDoPortal,
  Laudo,
  LaudoDoPortal,
  PacienteResumo,
  Recebimento,
  Sessao,
  Triagem,
} from '@vidacor/api-client';
import { ANTONIO, atendimentoDe, itensDaAgenda, SALAS, SESSAO, uuid } from './ficticios.ts';

function sessao(n: number, nome: string, perfis: string[]): Sessao {
  return { usuario: { id: uuid('0000000a', n), nome, perfis }, unidade: SESSAO.unidade, csrf: SESSAO.csrf };
}

export const SESSOES = {
  recepcao: SESSAO,
  tecnico: sessao(2, 'Bruno Sales', ['tecnico']),
  enfermagem: sessao(3, 'Lívia Prates', ['enfermagem']),
  medico: sessao(4, 'Dr. Paulo Medeiros', ['medico']),
  medicaTc: sessao(5, 'Dra. Cecília Souto', ['medico']),
  medicoSemVinculo: sessao(6, 'Dr. Marcos Freire', ['medico']),
  caixa: sessao(7, 'Carla Mendes', ['caixa']),
  gerente: sessao(8, 'Patrícia Lemos', ['coordenacao', 'gerente']),
  ti: sessao(9, 'Marcos Teixeira', ['ti']),
};

const SALA_1 = SALAS[0] as (typeof SALAS)[number];
const SALA_2 = SALAS[1] as (typeof SALAS)[number];
const SALA_6 = SALAS[5] as (typeof SALAS)[number];
export { SALA_1, SALA_2, SALA_6 };

const ITENS = itensDaAgenda();
function item(nome: string) {
  return ITENS.find((i) => i.paciente.nome === nome) as (typeof ITENS)[number];
}

export const SONIA = item('Sônia Maria Castro');
export const VERA = item('Vera Lúcia Antunes');
export const JULIANA = item('Juliana Sampaio Rocha');
export const MARIA_HELENA = item('Maria Helena Duarte');
export { ANTONIO };

/** Atendimento de Antônio depois do check-in: senha A012, pulseira ETQ-2026-018742. */
export const ATENDIMENTO_ANTONIO: AtendimentoDetalhadoFase1 = { ...atendimentoDe(ANTONIO, 1), id: uuid('0000000f', 501) };

export const ATENDIMENTO_VERA: AtendimentoDetalhadoFase1 = {
  ...atendimentoDe(VERA, 30),
  id: VERA.atendimento_id ?? uuid('0000000f', 30),
  senha: 'A007',
  pulseira: { codigo: 'ETQ-2026-018760', situacao: 'valida' },
  situacao: 'aguardando_sala',
};

/** Pulseiras e o que o servidor responde ao bipe (FC-PUL). */
export const PULSEIRAS = {
  antonio: 'ETQ-2026-018742',
  juliana: 'ETQ-2026-018755',
  invalidada: 'ETQ-2026-018700',
  vera: 'ETQ-2026-018760',
};

export const APARELHO_SALA_1 = { nome: 'Vivid E95', ae_title: 'VIVIDE95' };
export const APARELHO_SALA_6 = { nome: 'Tomógrafo Revolution', ae_title: 'REVOLUTION' };

function horario(hhmm: string, data = '2026-11-06'): string {
  return `${data}T${hhmm}:00-03:00`;
}

export function filaDaSala1(): FilaDaSala {
  return {
    sala: SALA_1,
    itens: [
      {
        atendimento_id: SONIA.atendimento_id ?? '',
        codigo: 'ATD-2026-118734',
        paciente: SONIA.paciente,
        exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma' },
        inicio: horario('08:00'),
        situacao: 'em_exame',
        requer_contraste: false,
        triagem_resultado: null,
      },
      {
        atendimento_id: ATENDIMENTO_ANTONIO.id,
        codigo: ATENDIMENTO_ANTONIO.codigo,
        paciente: ANTONIO.paciente,
        exame: { codigo_tuss: '40901300', nome: 'Eco e Doppler de carótidas' },
        inicio: horario('09:00'),
        situacao: 'aguardando_sala',
        requer_contraste: false,
        triagem_resultado: null,
      },
    ],
  };
}

export function filaDaSala6(triagem: Triagem | null): FilaDaSala {
  return {
    sala: SALA_6,
    itens: [
      {
        atendimento_id: ATENDIMENTO_VERA.id,
        codigo: ATENDIMENTO_VERA.codigo,
        paciente: VERA.paciente,
        exame: { codigo_tuss: '41001230', nome: 'TC de abdome com contraste' },
        inicio: horario('07:00'),
        situacao: 'aguardando_sala',
        requer_contraste: true,
        triagem_resultado: triagem === null ? null : triagem.resultado,
      },
    ],
  };
}

/** Triagem com creatinina 1,4 de 28/10/2026: TFG 40, precisa de aprovação médica (FC-TRI-02). */
export const TRIAGEM_PRECISA_APROVACAO: Triagem = {
  resultado: 'precisa_aprovacao_medica',
  tfg_calculada: 40,
  motivos: ['TFG 40 mL/min/1,73 m², abaixo de 45', 'Usa metformina'],
  regra_versao: 'TRI-2026.1',
  conduta: null,
};

export const TRIAGEM_LIBERADA: Triagem = {
  resultado: 'liberado',
  tfg_calculada: 92,
  motivos: [],
  regra_versao: 'TRI-2026.1',
  conduta: null,
};

/** Pacientes que a busca encontra (design/dados-ficticios.md). */
export const PACIENTES: PacienteResumo[] = [
  ANTONIO.paciente,
  { id: uuid('0000000e', 901), nome: 'Antônio Carlos Ribeiro', nascimento: '1980-01-02' },
  SONIA.paciente,
  VERA.paciente,
  JULIANA.paciente,
  MARIA_HELENA.paciente,
  { id: uuid('0000000e', 902), nome: 'Maria Helena Duarte', nascimento: '1968-03-21' },
];

export function horariosLivres(data: string): HorarioLivre[] {
  return ['08:30', '09:00', '09:30', '10:00', '10:30', '11:00'].map((h, i, todos) => ({
    inicio: horario(h, data),
    fim: horario(todos[i + 1] ?? '11:30', data),
  }));
}

export const GUIA_UNIMED = {
  operadora: 'Unimed Costa do Sol',
  numero: 'UN-55120',
  carteirinha: '0 064 000123456789 0',
  senha: 'UN-88231-X',
  validade: '2026-11-20',
  solicitante: 'Dr. Hugo Lemos, CRM 52-118734',
};

export const LAUDO_ANTONIO_ID = uuid('0000001a', 1);
export const LAUDO_SONIA_ID = uuid('0000001a', 2);

export function laudoAntonio(): Laudo {
  return {
    id: LAUDO_ANTONIO_ID,
    codigo: 'LAU-2026-004120',
    paciente: ANTONIO.paciente,
    exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' },
    situacao: 'rascunho',
    conversar_antes: false,
    publicado_no_portal: false,
    versao_vigente: 1,
    versoes: [
      {
        numero: 1,
        texto: 'Ventrículo esquerdo com dimensões normais.\nFração de ejeção de 66% (Teichholz).\nÁtrio esquerdo de 36 mm.',
        situacao: 'rascunho',
        motivo: null,
        assinada_por: null,
        assinada_em: null,
        hash_documento: null,
      },
    ],
    estudo: { accession: 'ACC-2026-052311', imagens: 42 },
  };
}

export function filaDeLaudos(): ItemDaFilaDeLaudos[] {
  return [
    {
      id: LAUDO_ANTONIO_ID,
      codigo: 'LAU-2026-004120',
      paciente: ANTONIO.paciente,
      exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' },
      situacao: 'rascunho',
      conversar_antes: false,
    },
    {
      id: LAUDO_SONIA_ID,
      codigo: 'LAU-2026-004118',
      paciente: SONIA.paciente,
      exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' },
      situacao: 'rascunho',
      conversar_antes: true,
    },
  ];
}

export const PORTAL_LAUDO_ECO: LaudoDoPortal = {
  id: uuid('0000001b', 1),
  codigo: 'LAU-2026-004118',
  exame: { codigo_tuss: '40901114', nome: 'Ecocardiograma transtorácico' },
  assinado_em: '2026-11-06T12:04:00Z',
  medico: 'Dr. Paulo Medeiros',
  retificado: true,
  versao: 2,
  texto: 'Ventrículo esquerdo com dimensões normais.\nÁtrio esquerdo de 38 mm.\n<b>sem negrito</b>',
};

export const PORTAL_LAUDO_DOPPLER: LaudoDoPortal = {
  id: uuid('0000001b', 2),
  codigo: 'LAU-2026-003990',
  exame: { codigo_tuss: '40901300', nome: 'Doppler colorido de carótidas' },
  assinado_em: '2025-03-14T13:30:00Z',
  medico: 'Dr. Marcos Freire',
  retificado: false,
  versao: 1,
  texto: 'Placa menor que 50% na carótida esquerda.',
};

export function itensDoPortal(): ItemDoPortal[] {
  return [PORTAL_LAUDO_ECO, PORTAL_LAUDO_DOPPLER].map((l) => ({ id: l.id, codigo: l.codigo, exame: l.exame, assinado_em: l.assinado_em, medico: l.medico, retificado: l.retificado, versao: l.versao }));
}

export function caixaAberto(): Caixa {
  return { id: uuid('0000001c', 1), codigo: 'CX-2026-000312', aberto_em: '2026-11-06T10:02:00Z', fundo_centavos: 20000, situacao: 'aberto' };
}

export function recebimentoSonia(descontoPb: number, forma: string): Recebimento {
  const subtotal = 38000;
  const desconto = Math.round((subtotal * descontoPb) / 10_000);
  return {
    id: uuid('0000001d', 1),
    codigo: 'REC-2026-004410',
    itens: [{ tuss: '40901114', descricao: 'Ecocardiograma transtorácico', valor_centavos: subtotal }],
    subtotal_centavos: subtotal,
    desconto_centavos: desconto,
    total_centavos: subtotal - desconto,
    tabela_versao: 'PART-12',
    situacao: 'aguardando_pagamento',
    pix: forma === 'pix' ? { txid: 'VC2026110600004410', copia_e_cola: '00020126580014br.gov.bcb.pix0136vidacor-ficticio-4410' } : null,
  };
}

export function contingenciaAberta(): Contingencia {
  return {
    id: uuid('0000001e', 7),
    codigo: 'CTG-2026-0007',
    nivel: 'total',
    motivo: 'servidor local fora do ar',
    responsavel: 'Patrícia Lemos',
    declarada_em: '2026-11-06T13:42:00Z',
    situacao: 'aberta',
  };
}
