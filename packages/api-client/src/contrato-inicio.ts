/** Início (Main.dc.html): o dia da unidade. Conferido em `conformidade.ts`. */

export interface ProximoPaciente {
  agendamento_id: string;
  inicio: string;
  nome: string;
  nascimento: string;
  exame: string;
  sala: string;
  situacao: string;
}

export interface OcupacaoDaSala {
  sala: string;
  minutos_marcados: number;
  minutos_abertos: number;
  percentual: number;
}

export interface CaixaDoDia {
  recebido_centavos: number;
  pix_centavos: number;
  cartao_centavos: number;
  dinheiro_centavos: number;
  aguardando_pagamento: number;
}

export interface InicioDoDia {
  data: string;
  exames_agendados: number;
  realizados: number;
  aguardando_agora: number;
  faltas: number;
  laudos_a_liberar: number;
  retificacoes_em_rascunho: number;
  reservas_sem_guia: number;
  proximos: ProximoPaciente[];
  ocupacao: OcupacaoDaSala[];
  caixa?: CaixaDoDia | null;
}
