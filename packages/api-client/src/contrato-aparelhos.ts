/** Aparelhos e integração DICOM (Aparelhos.dc.html). Só números, sem dado de paciente. Conferido em `conformidade.ts`. */

export interface AtividadeDoDia {
  listas_enviadas: number;
  listas_pendentes: number;
  listas_com_erro: number;
  ultimo_envio?: string | null;
  ultimo_erro?: string | null;
}

export interface AparelhoCadastrado {
  id: string;
  codigo: string;
  nome: string;
  ae_title: string;
  modalidade: string;
  sala_id: string;
  sala_nome: string;
  ativo: boolean;
  versao: number;
  situacao: 'conectado' | 'atencao' | 'desativado';
  hoje: AtividadeDoDia;
}

export interface PainelDosAparelhos {
  aparelhos: AparelhoCadastrado[];
  estudos_recebidos_hoje: number;
  estudos_sem_paciente: number;
}

export interface NovoAparelho {
  nome: string;
  ae_title: string;
  modalidade: string;
  sala_id: string;
  motivo: string;
}

export interface MudancaDeAparelho {
  tipo: 'desativar' | 'reativar';
  motivo: string;
}
