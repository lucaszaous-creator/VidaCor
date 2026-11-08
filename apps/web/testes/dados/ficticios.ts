/**
 * Dados FICTÍCIOS dos testes de tela, tirados de design/dados-ficticios.md e da constante `E` de
 * design/mockups/Agenda.dc.html (sexta, 06/11/2026). Nenhum dado real de paciente.
 * Só sintaxe apagável de TypeScript: este arquivo também roda direto no Node (teste Playwright).
 */
import type { AgendaFase1, AtendimentoDetalhadoFase1, ItemDaAgendaFase1, Sala, Sessao, SituacaoAgendamento } from '@vidacor/api-client';

export const DATA_DOS_TESTES = '2026-11-06';

export function uuid(prefixo: string, n: number): string {
  return `${prefixo}-0000-7000-8000-${String(n).padStart(12, '0')}`;
}

export const SESSAO: Sessao = {
  usuario: { id: uuid('0000000a', 1), nome: 'Ana Souza', perfis: ['recepcao'] },
  unidade: { id: uuid('0000000b', 1), nome: 'Unidade Centro' },
  csrf: 'csrf-ficticio-dos-testes',
};

export const SALAS: Sala[] = [
  'Sala 1, eco',
  'Sala 2, obstétrica',
  'Sala 3, USG e Doppler',
  'Sala 4, ergométrico',
  'Sala 5, raio X',
  'Sala 6, tomografia',
  'Posto MAPA e Holter',
].map((nome, i) => ({ id: uuid('0000000c', i + 1), nome }));

// [coluna da sala (2 a 8), linha (2 = 07:00), meias horas, estado do mockup, nome, nascimento, exame]
type Linha = [number, number, number, string, string, string, string];

const E: Linha[] = [
  [2, 2, 1, 'done', 'Maria Helena Duarte', '1968-03-12', 'Ecocardiograma'],
  [2, 3, 1, 'done', 'João Pedro Alves', '1971-09-04', 'Ecocardiograma'],
  [2, 4, 1, 'now', 'Sônia Maria Castro', '1962-02-03', 'Ecocardiograma'],
  [2, 6, 1, 'conf', 'Antônio Carlos Ribeiro', '1954-06-15', 'Eco e Doppler de carótidas'],
  [2, 7, 1, '', 'Cláudia Bastos Nunes', '1965-08-22', 'Eco com Doppler'],
  [2, 8, 1, '', 'Hélio Tavares Mendes', '1956-05-08', 'Ecocardiograma'],
  [2, 9, 1, '', 'Rosa Maria Teles', '1949-12-11', 'Ecocardiograma'],
  [2, 11, 1, '', 'Paulo Roberto Dias', '1960-01-27', 'Eco com Doppler'],
  [3, 2, 1, 'done', 'Beatriz Lima Cunha', '1995-07-19', 'USG obstétrica'],
  [3, 3, 2, 'now', 'Juliana Sampaio Rocha', '1993-04-12', 'USG morfológica'],
  [3, 5, 1, '', 'Patrícia Gomes Lopes', '1991-05-03', 'Obstétrica com Doppler'],
  [3, 7, 1, '', 'Fernanda Alves Prado', '1990-02-14', 'Translucência nucal'],
  [3, 9, 1, '', 'Carla Nogueira Souza', '1997-06-30', 'USG obstétrica'],
  [3, 16, 2, '', 'Aline Barros Ramos', '1992-10-05', 'USG morfológica'],
  [3, 19, 1, '', 'Mônica Silva Reis', '1988-03-21', 'USG obstétrica'],
  [4, 2, 1, 'done', 'Roberto Faria Lima', '1958-02-02', 'Doppler venoso'],
  [4, 3, 1, 'now', 'Ivone Rocha Teixeira', '1951-08-16', 'Doppler de carótidas'],
  [4, 6, 1, '', 'Marcelo Dias Faria', '1979-02-17', 'USG de abdome'],
  [4, 9, 1, '', 'Rita Costa Barbosa', '1966-09-09', 'USG de tireoide'],
  [4, 15, 1, '', 'Sérgio Alves Moura', '1963-04-25', 'Doppler venoso'],
  [4, 18, 1, '', 'Lia Martins Torres', '1975-11-13', 'USG de mamas'],
  [5, 14, 2, '', 'Eduardo Melo Vaz', '1977-09-21', 'Teste ergométrico'],
  [5, 16, 2, '', 'Simone Araújo Reis', '1969-06-06', 'Teste ergométrico'],
  [5, 18, 2, '', 'Gustavo Henrique Pires', '1970-01-14', 'Teste ergométrico'],
  [5, 21, 2, '', 'Ana Clara Lima', '1981-02-28', 'Teste ergométrico'],
  [6, 2, 1, 'done', 'Caio Vieira Moraes', '1985-10-10', 'Raio X de tórax'],
  [6, 4, 1, 'wait', 'Rafael Monteiro Lima', '1988-11-09', 'Raio X de tórax'],
  [6, 5, 1, '', 'Tiago Souza Braga', '1990-05-19', 'Raio X de tórax PA e perfil'],
  [6, 6, 1, 'cx', 'Laura Nunes Costa', '1983-10-30', 'Raio X de coluna lombar'],
  [7, 2, 2, 'now', 'Vera Lúcia Antunes', '1957-09-18', 'TC de abdome com contraste'],
  [7, 6, 2, 'wait', 'Lúcia Fernandes Prado', '1959-07-02', 'TC de crânio, revisar contraste'],
  [7, 10, 1, '', 'Otávio Ramos Sales', '1967-03-03', 'TC de tórax'],
  [7, 16, 2, '', 'Júlio Pereira Matos', '1972-12-12', 'TC de abdome'],
  [7, 20, 1, '', 'Elisa Campos Rocha', '1980-07-07', 'TC de crânio'],
  [8, 2, 1, 'done', 'Mário Jorge Pinto', '1955-01-15', 'Retirada de MAPA'],
  [8, 4, 1, 'falta', 'Neusa Almeida Freitas', '1961-04-22', 'Instalação de MAPA'],
  [8, 6, 1, '', 'Jorge Luiz Batista', '1959-08-01', 'Instalação de Holter'],
  [8, 22, 1, '', 'Renan Farias Alves', '1984-02-18', 'Instalação de MAPA'],
];

const SITUACAO: Record<string, SituacaoAgendamento> = {
  done: 'concluido',
  now: 'em_atendimento',
  wait: 'em_atendimento',
  conf: 'confirmado',
  cx: 'cancelado',
  falta: 'faltou',
  '': 'agendado',
};

function hora(linha: number): string {
  const minutos = 7 * 60 + (linha - 2) * 30;
  return `${String(Math.floor(minutos / 60)).padStart(2, '0')}:${String(minutos % 60).padStart(2, '0')}`;
}

export function itensDaAgenda(): ItemDaAgendaFase1[] {
  return E.map(([coluna, linha, meias, estado, nome, nascimento, exame], i) => {
    const sala = SALAS[coluna - 2] as Sala;
    const situacao = SITUACAO[estado] ?? 'agendado';
    const comAtendimento = estado === 'done' || estado === 'now' || estado === 'wait';
    return {
      id: uuid('0000000d', i + 1),
      codigo: `AG-2611-${String(4100 + i * 7)}`,
      inicio: `${DATA_DOS_TESTES}T${hora(linha)}:00-03:00`,
      fim: `${DATA_DOS_TESTES}T${hora(linha + meias)}:00-03:00`,
      sala,
      paciente: { id: uuid('0000000e', i + 1), nome, nascimento },
      exame: { codigo_tuss: '40901114', nome: exame },
      situacao,
      atendimento_id: comAtendimento ? uuid('0000000f', i + 1) : null,
      // FC-CHK-04: outro Antônio Carlos Ribeiro, nascido em 02/01/1980, também tem exame hoje.
      homonimos: nome === 'Antônio Carlos Ribeiro' ? [{ id: uuid('0000000e', 901), nome, nascimento: '1980-01-02' }] : [],
      requer_contraste: exame.includes('contraste'),
      laudo_situacao: estado === 'done' ? 'rascunho' : null,
    };
  });
}

export function agendaDoDia(data: string, salaId?: string | null): AgendaFase1 {
  if (data !== DATA_DOS_TESTES) {
    return { data, itens: [] };
  }
  const itens = itensDaAgenda().filter((i) => salaId === undefined || salaId === null || i.sala.id === salaId);
  return { data, itens };
}

export const ANTONIO = itensDaAgenda().find((i) => i.paciente.nome === 'Antônio Carlos Ribeiro') as ItemDaAgendaFase1;

export function atendimentoDe(item: ItemDaAgendaFase1, sequencia: number, comEstudo = false): AtendimentoDetalhadoFase1 {
  return {
    id: item.atendimento_id ?? uuid('0000000f', 900 + sequencia),
    codigo: `ATD-2026-${String(118790 + sequencia).padStart(6, '0')}`,
    accession: `ACC-2026-${String(52310 + sequencia).padStart(6, '0')}`,
    agendamento_id: item.id,
    paciente: item.paciente,
    sala: item.sala,
    requer_contraste: item.requer_contraste,
    situacao: 'aguardando_sala',
    senha: `A${String(11 + sequencia).padStart(3, '0')}`,
    pulseira: { codigo: `ETQ-2026-${String(18741 + sequencia).padStart(6, '0')}`, situacao: 'valida', tipo: 'pulseira' },
    criado_em: `${DATA_DOS_TESTES}T12:01:00Z`,
    triagem: null,
    laudo_situacao: null,
    estudo: comEstudo ? { study_uid: '1.2.826.0.1.3680043.8.498.1', imagens: 24, recebido_em: `${DATA_DOS_TESTES}T12:20:00Z` } : null,
  };
}
