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
