/** Busca global (Busca-Global.dc.html). Conferido em `conformidade.ts`. */

export interface ResultadoDaBusca {
  tipo: 'paciente' | 'agendamento' | 'atendimento' | 'etiqueta' | 'laudo';
  id: string;
  titulo: string;
  detalhe: string;
  destino: string;
}
