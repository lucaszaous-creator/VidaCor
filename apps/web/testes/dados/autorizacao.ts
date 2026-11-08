/**
 * Autorizações fictícias (Autorizacao.dc.html), como o servidor devolveria. Só sintaxe apagável de
 * TypeScript (roda também no Node, na verificação de que a tela cabe na janela).
 */
import type { AutorizacaoDetalhe, AutorizacaoResumo, SituacaoDaAutorizacao } from '@vidacor/api-client';
import { uuid } from './ficticios.ts';

function resumo(n: number, nome: string, nascimento: string, operadora: string, canal: AutorizacaoResumo['canal'], situacao: SituacaoDaAutorizacao, extra: Partial<AutorizacaoResumo> = {}): AutorizacaoResumo {
  return {
    id: uuid('000000a7', n),
    codigo: `AUT-2026-${String(n).padStart(6, '0')}`,
    paciente: { nome, nascimento },
    operadora,
    canal,
    situacao,
    procedimentos: ['40901114 Ecocardiograma transtorácico', '40901300 Ecodoppler de carótidas'],
    protocolo: null,
    motivo: null,
    agendamento_id: uuid('000000a8', n),
    inicio_do_exame: '2026-11-12T11:30:00Z',
    criado_em: '2026-11-08T19:05:00Z',
    ...extra,
  };
}

function detalhe(r: AutorizacaoResumo, extra: Partial<AutorizacaoDetalhe> = {}): AutorizacaoDetalhe {
  return {
    resumo: r,
    carteirinha: '0123.4567',
    solicitante: 'Dr. Júlio Prado, conselho 06 52123456, UF 33',
    indicacao_clinica: null,
    enviada_em: r.situacao === 'aguardando_portal' ? null : '2026-11-08T19:05:00Z',
    respondida_em: null,
    itens: [
      { tuss: '40901114', descricao: 'Ecocardiograma transtorácico', solicitado: 1, autorizado: null },
      { tuss: '40901300', descricao: 'Ecodoppler de carótidas', solicitado: 1, autorizado: null },
    ],
    respostas: [],
    guia: null,
    pendente_no_outbox: null,
    situacao_do_agendamento: 'reserva_provisoria',
    ...extra,
  };
}

const RESPOSTA = { origem: 'webservice' as const, sha256: 'c0ffee'.padEnd(64, '0'), hash_confere: true, aplicada: true, recebida_em: '2026-11-08T20:40:00Z', erro_leitura: null };

/** Estado inicial das autorizações (o simulador muda conforme a tela pede). */
export function autorizacoesIniciais(): AutorizacaoDetalhe[] {
  const autorizada = resumo(1, 'Antônio Carlos Ribeiro', '1954-06-15', 'Unimed', 'webservice', 'autorizada', { protocolo: 'OP2026000001' });
  const analise = resumo(2, 'Vera Lúcia Antunes', '1957-09-18', 'Unimed', 'webservice', 'em_analise');
  const negada = resumo(3, 'Maria Helena Duarte', '1968-03-12', 'Unimed', 'webservice', 'negada', { motivo: 'Código(s) da operadora: 1423. Procedimento fora da cobertura do plano' });
  const portal = resumo(4, 'Sônia Maria Castro', '1962-02-03', 'Petrobras', 'portal', 'aguardando_portal');
  return [
    detalhe(negada, { respostas: [{ ...RESPOSTA, situacao_lida: 'negada' }], respondida_em: '2026-11-08T20:40:00Z' }),
    detalhe(portal, { situacao_do_agendamento: 'reserva_provisoria' }),
    detalhe(analise, {
      respostas: [{ ...RESPOSTA, situacao_lida: 'em_analise' }],
      pendente_no_outbox: { tentativas: 2, proxima_em: '2026-11-08T21:05:00Z', ultimo_erro: 'a operadora ainda está analisando o pedido' },
    }),
    detalhe(autorizada, {
      respostas: [{ ...RESPOSTA, situacao_lida: 'autorizada' }],
      respondida_em: '2026-11-08T20:40:00Z',
      itens: [
        { tuss: '40901114', descricao: 'Ecocardiograma transtorácico', solicitado: 1, autorizado: 1 },
        { tuss: '40901300', descricao: 'Ecodoppler de carótidas', solicitado: 1, autorizado: 1 },
      ],
      guia: { id: uuid('000000a9', 1), codigo: 'GUI-2026-000077', numero: 'OP2026000001', senha: 'DEV-2026000001', validade: '2026-12-08', situacao: 'consumida' },
      situacao_do_agendamento: 'confirmado',
    }),
  ];
}
