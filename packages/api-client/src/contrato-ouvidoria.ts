/** Ouvidoria (Ouvidoria.dc.html). Prazo calculado pelo servidor. Conferido em `conformidade.ts`. */

export type TipoDeManifestacao = 'reclamacao' | 'elogio' | 'sugestao' | 'denuncia' | 'lgpd';
export type CanalDeManifestacao = 'telefone' | 'presencial' | 'email' | 'portal';

export interface NovaManifestacao {
  tipo: TipoDeManifestacao;
  canal: CanalDeManifestacao;
  paciente_id?: string | null;
  nome_informado?: string | null;
  anonima?: boolean;
  relato: string;
}

export interface ProtocoloDaManifestacao {
  id: string;
  codigo: string;
  prazo_em: string;
}

export interface Manifestacao {
  id: string;
  codigo: string;
  tipo: string;
  canal: string;
  quem?: string | null;
  nascimento?: string | null;
  anonima: boolean;
  relato: string;
  recebida_em: string;
  prazo_em: string;
  situacao: 'aberta' | 'respondida';
  vencida: boolean;
  resposta?: string | null;
  respondida_em?: string | null;
}

export interface PainelDaOuvidoria {
  abertas: number;
  vencem_hoje: number;
  no_prazo_percentual?: number | null;
  manifestacoes: Manifestacao[];
}
