/** Etiquetas e rastreio (Etiquetas.dc.html, docs/20). Estado e histórico calculados pelo núcleo. Conferido em `conformidade.ts`. */

export type EstadoDaEtiqueta = 'impressa' | 'em_uso' | 'invalidada' | 'divergencia';

export interface PacienteDaEtiqueta {
  id: string;
  nome: string;
  nascimento: string;
}

export interface LinhaDeEtiqueta {
  codigo: string;
  tipo: string;
  estado: EstadoDaEtiqueta;
  paciente: PacienteDaEtiqueta;
  exame: string;
  sala: string;
  accession: string;
  motivo_invalidacao?: string | null;
  substituida_por?: string | null;
  substitui?: string | null;
  ultimo_evento?: { descricao: string; em: string } | null;
}

export interface DivergenciaDeSala {
  id: string;
  codigo: string;
  etiqueta: string;
  paciente: PacienteDaEtiqueta;
  exame: string;
  sala_bipada: string;
  sala_correta: string;
  sala_correta_id: string;
  aberta_em: string;
}

export interface PainelDeEtiquetas {
  dia: string;
  impressas: number;
  bipadas: number;
  reimpressoes: number;
  invalidadas: number;
  divergencias_abertas: number;
  divergencias: DivergenciaDeSala[];
  etiquetas: LinhaDeEtiqueta[];
}

export interface EventoDaEtiqueta {
  tipo: 'impressa' | 'invalidada' | 'bipada' | 'bloqueada' | 'resolvida' | 'outro';
  descricao: string;
  em: string;
  por?: string | null;
  nivel: 'ok' | 'alerta';
}

export interface DetalheDaEtiqueta {
  etiqueta: LinhaDeEtiqueta;
  eventos: EventoDaEtiqueta[];
}

export interface EtiquetaInvalidada {
  codigo: string;
  situacao: string;
  motivo_invalidacao: string;
}

export interface PedidoResolucaoDeSala {
  resolucao: 'encaminhado' | 'outro';
  observacao?: string | null;
}

export interface DivergenciaResolvida {
  id: string;
  codigo: string;
  situacao: string;
  descricao: string;
}
