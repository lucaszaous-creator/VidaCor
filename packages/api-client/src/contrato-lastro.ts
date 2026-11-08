/** Explorador de lastro (docs/21). Conferido em `conformidade.ts`. */
import type { PacienteResumo } from './contrato-m0';

export interface NoDaCadeia {
  tipo: string;
  id: string;
  codigo: string;
  situacao?: string | null;
  criado_em: string;
}

export interface EventoDaCadeia {
  sequencia: number;
  dominio: string;
  tipo: string;
  registro_codigo?: string | null;
  criado_em: string;
  origem: string;
  hash: string;
  integro: boolean;
}

export interface CadeiaDoAtendimento {
  paciente: PacienteResumo;
  atendimento_codigo: string;
  nos: NoDaCadeia[];
  eventos: EventoDaCadeia[];
  integra: boolean;
}

/** Pedido do dossiê em PDF: qualquer código do atendimento, como na busca. */
export interface PedidoDossie {
  codigo: string;
}

/** Documento binário baixado do núcleo (laudo impresso, dossiê). */
export interface DocumentoBaixado {
  conteudo: Blob;
  /** Nome sugerido pelo servidor, só com códigos (nunca nome do paciente). */
  nomeDoArquivo: string;
  /** SHA-256 dos bytes, calculado e registrado pelo servidor. */
  sha256: string;
  /** Código da emissão registrada (DOC-… ou DOS-…). */
  codigoDaEmissao: string;
}
