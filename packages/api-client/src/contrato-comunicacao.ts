/** Avisos por WhatsApp (migrations 0041 e 0074). Conferido em `conformidade.ts`. */
export type ModeloDeAviso = 'confirmacao' | 'laudo_pronto' | 'reserva_expirada' | 'oferta_vaga';
export type SituacaoDaMensagem = 'enviada' | 'bloqueada_opt_out' | 'sem_celular' | 'modelo_desligado';
export interface MensagemAoPaciente {
  modelo: ModeloDeAviso;
  situacao: SituacaoDaMensagem;
  resposta?: string | null;
  criado_em: string;
}
export interface ComunicacaoDoPaciente {
  whatsapp: boolean;
  motivo?: string | null;
  mensagens: MensagemAoPaciente[];
}
export interface PreferenciaWhatsapp {
  whatsapp: boolean;
  motivo: string;
}
export interface LembretesEnfileirados {
  enfileirados: number;
}

/** Comunicação com pacientes (Comunicacao.dc.html), corte do MVP: registro e avisos automáticos. */
export interface MensagemDoRegistro {
  id: string;
  paciente_id: string;
  nome: string;
  nascimento: string;
  modelo: ModeloDeAviso;
  situacao: SituacaoDaMensagem;
  resposta?: string | null;
  criado_em: string;
}

export interface AvisoAutomatico {
  modelo: ModeloDeAviso;
  nome: string;
  descricao: string;
  ativo: boolean;
  motivo?: string | null;
  alterado_em?: string | null;
  alterado_por?: string | null;
  versao_id?: string | null;
}

export interface PainelDeComunicacao {
  whatsapp_configurado: boolean;
  hoje: { enviadas: number; confirmaram: number; opt_out: number; sem_celular: number; desligadas: number };
  mensagens: MensagemDoRegistro[];
  automaticas: AvisoAutomatico[];
  pacientes_sem_whatsapp: number;
}

export interface MudancaDeAviso {
  ativo: boolean;
  motivo: string;
  versao_vista: string | null;
}

export type FiltroDeMensagens = 'todas' | 'nao_entregues' | 'confirmaram';
