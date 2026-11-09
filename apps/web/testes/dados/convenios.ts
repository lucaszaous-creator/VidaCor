/**
 * Convênios e tabelas e Check-up e pacotes fictícios (Convenios-Tabelas.dc.html e
 * Checkup-Pacotes.dc.html), com os valores que o servidor devolveria: a tela não calcula nada. Só
 * sintaxe apagável de TypeScript (roda também no Node, na verificação de que a tela cabe na janela).
 */
import type { CatalogoDePacotes, ConvenioDetalhe, ConvenioResumo, PrecosDoConvenio, PropostaDeTabela, TermosDoConvenio, VersaoDoPacoteDeCheckup } from '@vidacor/api-client';
import { uuid } from './ficticios.ts';

export const UNIMED_ID = uuid('0000003c', 1);
export const SULAMERICA_ID = uuid('0000003c', 2);
export const RITA_ID = uuid('0000000a', 11);

const CONJUNTOS = [
  { versao: 'AUD-TISS-2026.1', nome: 'Padrão TISS (operadora sem regras próprias)' },
  { versao: 'AUD-UNIMED-2026.1', nome: 'Unimed' },
  { versao: 'AUD-SULAMERICA-2026.1', nome: 'SulAmérica' },
  { versao: 'AUD-BRADESCO-2026.1', nome: 'Bradesco Saúde' },
];

export function termosUnimed(): TermosDoConvenio {
  return {
    codigo_prestador: '0004567',
    nome_prestador: 'VidaCor Imagem Unidade Centro',
    cnes: '1234567',
    regra_versao: 'AUD-UNIMED-2026.1',
    prazo_envio_dias: 30,
    prazo_recurso_dias: 30,
    prazo_pagamento_dias: 30,
    vigencia_inicio: '2021-01-01',
    vigencia_fim: '2027-12-31',
    reajuste: 'IPCA em janeiro',
    contato_nome: 'Fernanda Ruas',
    contato_telefone: '(48) 3000-1100',
    contato_email: 'prestador@unimed.exemplo',
    autorizacao_por_webservice: true,
    ativo: true,
  };
}

export function propostaUnimed(): PropostaDeTabela {
  return {
    id: uuid('0000003d', 1),
    codigo: 'PTB-2026-000004',
    contrato_id: UNIMED_ID,
    tipo: 'reajuste',
    reajuste_pontos_base: 520,
    base_versao: 14,
    vigente_desde: '2027-01-01',
    descricao: 'Unimed 2027',
    documento: 'Carta de reajuste 2027',
    motivo: 'reajuste anual pelo IPCA',
    situacao: 'pendente',
    proposta_por: 'Rita Faturamento',
    proposta_por_id: RITA_ID,
    proposta_em: '2026-11-06T13:00:00Z',
    decidido_por: null,
    decidido_em: null,
    motivo_decisao: null,
    itens: 142,
    total_antes_centavos: 1_843_000,
    total_depois_centavos: 1_938_836,
    maiores: [
      { tuss: '41001230', descricao: 'TC de abdome com contraste', antes_centavos: 61_200, depois_centavos: 64_382 },
      { tuss: '40901114', descricao: 'Ecocardiograma transtorácico', antes_centavos: 24_860, depois_centavos: 26_153 },
      { tuss: '40901300', descricao: 'USG morfológica do 2º trimestre', antes_centavos: 21_460, depois_centavos: 22_576 },
    ],
    tabela_versao: null,
  };
}

export function convenioUnimed(comProposta = false): ConvenioDetalhe {
  return {
    id: UNIMED_ID,
    codigo: 'CVN-2026-000001',
    operadora: 'Unimed',
    registro_ans: '345678',
    situacao: 'vigente',
    versao: 3,
    termos: termosUnimed(),
    tabelas: [
      { id: uuid('0000003e', 14), codigo: 'TBC-2026-000014', versao: 14, vigente_desde: '2026-01-01', descricao: 'CBHPM 5ª ed. mais 10%', motivo: 'reajuste anual pelo IPCA', itens: 142, proposta_por: 'Rita Faturamento', aprovada_por: 'Patrícia Lemos', criada_em: '2025-12-10T12:00:00Z', situacao: 'vigente' },
      { id: uuid('0000003e', 13), codigo: 'TBC-2025-000013', versao: 13, vigente_desde: '2025-01-01', descricao: 'CBHPM 5ª ed. mais 10%', motivo: null, itens: 138, proposta_por: null, aprovada_por: null, criada_em: '2024-12-10T12:00:00Z', situacao: 'anterior' },
    ],
    proposta_pendente: comProposta ? propostaUnimed() : null,
    regras_autorizacao: {
      codigo: 'RAU-2026-000005',
      versao_regras: 5,
      motivo: 'manual do prestador 2026',
      publicada_por: 'Rita Faturamento',
      publicada_em: '2026-09-02T12:00:00Z',
      itens: [
        { tuss: '41001230', nome: 'TC de abdome com contraste', exige_senha: true, validade_dias: 30, documentos: 'Pedido médico com CID e relatório clínico' },
        { tuss: '40901220', nome: 'Ecocardiograma com estresse', exige_senha: true, validade_dias: 30, documentos: 'Pedido médico com CID' },
        { tuss: '40801012', nome: 'Raio X de tórax', exige_senha: false, validade_dias: null, documentos: 'Pedido médico e carteirinha' },
      ],
    },
    auditoria: {
      versao: 'AUD-UNIMED-2026.1',
      nome: 'Unimed',
      regras: [
        { regra: 'operadora_sem_contrato', titulo: 'Operadora sem contrato', efeito: 'bloqueia', origem: 'regra da clínica' },
        { regra: 'senha_vencida', titulo: 'Atendimento depois da validade da senha', efeito: 'bloqueia', origem: 'padrão TISS' },
        { regra: 'solicitante_incompleto', titulo: 'Solicitante incompleto', efeito: 'corrigir_antes', origem: 'padrão TISS' },
        { regra: 'prazo_de_envio_proximo', titulo: 'Prazo de envio próximo', efeito: 'atencao', origem: 'contrato' },
      ],
      disponiveis: CONJUNTOS,
    },
    historico: [
      { codigo: 'MCV-2026-000003', tipo: 'alterar', motivo: 'aditivo 03, inclusão de tomografia', versao_do_contrato: 2, antes: termosUnimed(), depois: termosUnimed(), quem: 'Rita Faturamento', em: '2026-06-01T12:00:00Z' },
      { codigo: 'MCV-2026-000001', tipo: 'cadastrar', motivo: 'credenciamento assinado em 2021', versao_do_contrato: 1, antes: null, depois: termosUnimed(), quem: 'Rita Faturamento', em: '2026-01-12T12:00:00Z' },
    ],
  };
}

export function conveniosIniciais(comProposta = false): ConvenioResumo[] {
  return [
    { id: UNIMED_ID, codigo: 'CVN-2026-000001', operadora: 'Unimed', registro_ans: '345678', situacao: 'vigente', vigencia_fim: '2027-12-31', tabela_versao: 14, proposta_pendente: comProposta },
    { id: SULAMERICA_ID, codigo: 'CVN-2026-000002', operadora: 'SulAmérica', registro_ans: '006043', situacao: 'renovar', vigencia_fim: '2026-12-31', tabela_versao: 6, proposta_pendente: false },
  ];
}

export function precosUnimed(quantos = 6): PrecosDoConvenio {
  const base: [string, string, number][] = [
    ['40901114', 'Ecocardiograma transtorácico', 24_860],
    ['40901238', 'Doppler de carótidas e vertebrais', 19_620],
    ['40901300', 'USG morfológica do 2º trimestre', 21_460],
    ['40901203', 'USG obstétrica', 11_840],
    ['40805026', 'Raio X de tórax PA e perfil', 6_210],
    ['41001230', 'TC de abdome com contraste', 61_200],
  ];
  const itens = Array.from({ length: quantos }, (_, k) => {
    const [tuss, descricao, v] = base[k % base.length] ?? ['40901114', 'Exame', 10_000];
    return { tuss: k < base.length ? tuss : String(41000000 + k), descricao: k < base.length ? descricao : `${descricao} com nome comprido de exame fictício ${String(k)}`, valor_centavos: v, valor_anterior_centavos: Math.round(v / 1.048) };
  });
  return { versao: 14, vigente_desde: '2026-01-01', descricao: 'CBHPM 5ª ed. mais 10%', itens };
}

function versaoCardio(regra: number, situacao: VersaoDoPacoteDeCheckup['situacao'], preco: number, desde: string): VersaoDoPacoteDeCheckup {
  const avulso = 89_000;
  return {
    id: uuid('0000003f', regra),
    regra_versao: regra,
    nome: 'Check-up cardiológico',
    para_quem: 'Para quem tem fator de risco ou mais de 40 anos; um pedido médico cobre os três exames',
    vigente_desde: desde,
    vigente_ate: null,
    preco_centavos: preco,
    motivo: 'pacote da cardiologia',
    publicada_por: 'Patrícia Lemos',
    publicada_em: '2026-09-20T12:00:00Z',
    situacao,
    avulso_centavos: avulso,
    economia_centavos: avulso - preco,
    desconto_pontos_base: Math.floor(((avulso - preco) * 10_000) / avulso),
    vale_no_orcamento: preco < avulso,
    duracao_total_min: 125,
    exames: [
      { ordem: 1, tuss: '40901300', nome: 'Ecodoppler de carótidas', duracao_min: 40, preparo: 'Sem preparo', porque: 'Primeiro: sem preparo e libera a sala 3 cedo.', avulso_centavos: 32_000 },
      { ordem: 2, tuss: '40901114', nome: 'Ecocardiograma transtorácico', duracao_min: 40, preparo: 'Sem preparo', porque: 'Antes do esforço, com o coração em repouso.', avulso_centavos: 38_000 },
      { ordem: 3, tuss: '40901220', nome: 'Ecocardiograma com estresse', duracao_min: 45, preparo: 'Tênis e roupa leve; refeição leve até 2 h antes', porque: 'Por último: o esforço muda frequência e pressão.', avulso_centavos: 19_000 },
    ],
  };
}

export function pacotesIniciais(extras = 0): CatalogoDePacotes {
  const mulher = versaoCardio(1, 'vigente', 76_000, '2026-08-01');
  return {
    desconto_maximo_pontos_base: 4_000,
    pacotes: [
      { codigo: 'PCT-2026-000001', nome: 'Check-up cardiológico', versoes: [versaoCardio(4, 'futura', 72_000, '2027-01-01'), versaoCardio(3, 'vigente', 69_000, '2026-10-01')] },
      {
        codigo: 'PCT-2026-000002',
        nome: 'Check-up da mulher',
        versoes: [{ ...mulher, id: uuid('0000003f', 20), nome: 'Check-up da mulher', para_quem: 'Rastreio anual com USG de mamas, transvaginal, tireoide e abdome', avulso_centavos: 70_000, economia_centavos: null, desconto_pontos_base: null, vale_no_orcamento: false }],
      },
      ...Array.from({ length: extras }, (_, k) => ({ codigo: `PCT-2026-0001${String(10 + k)}`, nome: `Pacote fictício número ${String(k)} com nome comprido`, versoes: [{ ...versaoCardio(1, 'vigente', 69_000, '2026-10-01'), id: uuid('0000003f', 100 + k) }] })),
    ],
  };
}
