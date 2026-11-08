/**
 * Faturamento TISS fictício (Faturamento.dc.html e Auditoria-Faturamento.dc.html), com os valores
 * que o servidor devolveria: a tela não calcula nada. Só sintaxe apagável de TypeScript (roda também
 * no Node, na verificação de que a tela cabe na janela). Dados de design/dados-ficticios.md.
 */
import type {
  ContaDetalhe,
  ContaResumo,
  ContratoDeConvenio,
  Glosa,
  LoteTiss,
  PainelAuditoria,
  PendenciaDaConta,
} from '@vidacor/api-client';
import { uuid } from './ficticios.ts';

export const CONTRATO_UNIMED: ContratoDeConvenio = {
  id: uuid('000000f1', 1),
  codigo: 'CTO-2026-000001',
  operadora: 'Unimed',
  registro_ans: '123456',
  regra_versao: 'AUD-UNIMED-2026.1',
  prazo_envio_dias: 30,
  prazo_recurso_dias: 30,
  contas_prontas: 1,
};

const ECO = '40901114 Ecocardiograma transtorácico';

function conta(n: number, nome: string, nascimento: string, guia: string, situacao: ContaResumo['situacao'], impedem: number): ContaResumo {
  return {
    id: uuid('000000f2', n),
    codigo: `FAT-2026-${String(100 + n).padStart(6, '0')}`,
    paciente: { nome, nascimento },
    operadora: 'Unimed',
    numero_guia_operadora: guia,
    procedimentos: [ECO],
    valor_total_centavos: 24_860,
    situacao,
    data_execucao: '2026-11-05',
    enviar_ate: '2026-12-05',
    pendencias_que_impedem: impedem,
    pendencias_de_atencao: 0,
    regra_versao: 'AUD-UNIMED-2026.1',
    lote_codigo: null,
  };
}

const LAUDO: PendenciaDaConta = {
  regra: 'laudo-nao-assinado',
  regra_versao: 'AUD-UNIMED-2026.1',
  efeito: 'bloqueia',
  origem: 'regra da clínica',
  titulo: 'Laudo não assinado',
  detalhe: 'O laudo deste exame ainda não foi assinado.',
  o_que_fazer: 'Avise o médico responsável pelo laudo.',
};
const SOLICITANTE: PendenciaDaConta = {
  regra: 'solicitante-incompleto',
  regra_versao: 'AUD-UNIMED-2026.1',
  efeito: 'corrigir_antes',
  origem: 'padrão TISS',
  titulo: 'Dados do médico solicitante incompletos',
  detalhe: 'Solicitante Dr. Júlio Prado sem conselho, número, UF ou CBO.',
  o_que_fazer: 'Preencha conselho, número, UF e CBO do solicitante a partir do pedido médico.',
};

export interface EstadoDoFaturamento {
  contas: ContaDetalhe[];
  lotes: LoteTiss[];
  glosas: Glosa[];
}

function detalhe(resumo: ContaResumo, pendencias: PendenciaDaConta[], completo: boolean): ContaDetalhe {
  return {
    resumo,
    carteirinha: '0 123 4567',
    senha: 'UN-88231-X',
    validade_senha: '2026-11-30',
    solicitante: completo
      ? { nome: 'Dr. Júlio Prado', conselho: '06', numero: '52123456', uf: '33', cbos: '225125' }
      : { nome: 'Dr. Júlio Prado', conselho: null, numero: null, uf: null, cbos: null },
    cid: null,
    assinatura_beneficiario: false,
    itens: [{ sequencial: 1, tuss: '40901114', descricao: 'Ecocardiograma transtorácico', quantidade: 1, valor_unitario_centavos: 24_860 }],
    pendencias,
    historico: [],
    versao: 2,
    glosas: [],
  };
}

/** Estado inicial: Antônio pronto, Maria com laudo pendente e solicitante incompleto, um lote enviado com glosa. */
export function faturamentoInicial(): EstadoDoFaturamento {
  const antonio = conta(1, 'Antônio Carlos Ribeiro', '1954-06-15', 'UN-55120', 'pronta', 0);
  const maria = conta(2, 'Maria Helena Duarte', '1968-03-12', 'UN-55121', 'com_pendencia', 2);
  const sonia: ContaResumo = {
    ...conta(3, 'Sônia Maria Castro', '1962-02-03', 'UN-55090', 'paga_com_glosa', 0),
    data_execucao: '2026-10-20',
    lote_codigo: 'LTI-2026-000001',
  };
  const lote: LoteTiss = {
    id: uuid('000000f3', 1),
    codigo: 'LTI-2026-000001',
    numero: 148,
    contrato_id: CONTRATO_UNIMED.id,
    operadora: 'Unimed',
    quantidade_guias: 1,
    valor_total_centavos: 24_860,
    situacao: 'demonstrativo_recebido',
    versao_tiss: '4.03.00',
    hash_tiss: '0f5a3c1b2d4e6f708192a3b4c5d6e7f8',
    sha256: 'a'.repeat(64),
    protocolo: 'UN-0098123',
    criado_em: '2026-10-28T13:00:00Z',
    enviado_em: '2026-10-28T13:05:00Z',
    envio_pendente: null,
    valor_pago_centavos: 18_000,
    valor_glosado_centavos: 6_860,
  };
  const glosa: Glosa = {
    id: uuid('000000f4', 1),
    codigo: 'GLO-2026-000001',
    conta_id: sonia.id,
    conta_codigo: sonia.codigo,
    paciente: sonia.paciente,
    operadora: 'Unimed',
    codigo_glosa: '1801',
    motivo: 'Procedimento sem relatório clínico',
    valor_centavos: 6_860,
    prazo_recurso: '2026-12-05',
    situacao: 'aberta',
    recurso: null,
  };
  return {
    contas: [detalhe(antonio, [], true), detalhe(maria, [LAUDO, SOLICITANTE], false), { ...detalhe(sonia, [], true), glosas: [glosa] }],
    lotes: [lote],
    glosas: [glosa],
  };
}

/** Painel da auditoria a partir do estado (o servidor agrupa as pendências por regra). */
export function painelDe(e: EstadoDoFaturamento): PainelAuditoria {
  const abertas = e.contas.filter((c) => ['pronta', 'com_pendencia', 'em_conferencia'].includes(c.resumo.situacao));
  const grupos = new Map<string, PainelAuditoria['grupos'][number]>();
  for (const c of abertas) {
    for (const p of c.pendencias) {
      const g = grupos.get(p.regra) ?? { regra: p.regra, titulo: p.titulo, efeito: p.efeito, origem: p.origem, o_que_fazer: p.o_que_fazer, regra_versoes: [p.regra_versao], contas: [] };
      g.contas.push({ conta_id: c.resumo.id, codigo: c.resumo.codigo, paciente: c.resumo.paciente, operadora: c.resumo.operadora, numero_guia_operadora: c.resumo.numero_guia_operadora, detalhe: p.detalhe });
      grupos.set(p.regra, g);
    }
  }
  const bloqueadas = abertas.filter((c) => c.pendencias.some((p) => p.efeito === 'bloqueia')).length;
  const comPendencia = abertas.filter((c) => c.resumo.situacao === 'com_pendencia').length;
  return {
    resumo: {
      conferidas: abertas.length,
      prontas: abertas.filter((c) => c.resumo.situacao === 'pronta').length,
      corrigir_antes: comPendencia - bloqueadas,
      bloqueadas,
    },
    grupos: [...grupos.values()].sort((a, b) => (a.efeito === b.efeito ? 0 : a.efeito === 'bloqueia' ? -1 : 1)),
    conjuntos: [
      {
        versao: 'AUD-UNIMED-2026.1',
        nome: 'Unimed',
        operadoras: ['Unimed'],
        regras: [
          { regra: 'senha-vencida', titulo: 'Senha vencida antes da data do exame', efeito: 'bloqueia', origem: 'padrão TISS' },
          { regra: 'laudo-nao-assinado', titulo: 'Laudo não assinado', efeito: 'bloqueia', origem: 'regra da clínica' },
          { regra: 'solicitante-incompleto', titulo: 'Dados do médico solicitante incompletos', efeito: 'corrigir_antes', origem: 'padrão TISS' },
          { regra: 'valor-diferente-da-tabela', titulo: 'Valor diferente da tabela vigente', efeito: 'corrigir_antes', origem: 'contrato' },
          { regra: 'prazo-de-envio-proximo', titulo: 'Prazo de envio vence em até 7 dias', efeito: 'atencao', origem: 'contrato' },
        ],
      },
      {
        versao: 'AUD-SULAMERICA-2026.1',
        nome: 'SulAmérica',
        operadoras: [],
        regras: [{ regra: 'cid-ausente', titulo: 'CID-10 obrigatório ausente', efeito: 'corrigir_antes', origem: 'manual do prestador' }],
      },
    ],
  };
}
