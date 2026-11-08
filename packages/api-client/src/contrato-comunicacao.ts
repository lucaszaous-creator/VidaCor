/** Avisos por WhatsApp (migration 0041). Conferido em `conformidade.ts`. */
export interface MensagemAoPaciente {
  modelo: 'confirmacao' | 'laudo_pronto' | 'reserva_expirada' | string;
  situacao: 'enviada' | 'bloqueada_opt_out' | 'sem_celular' | string;
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
