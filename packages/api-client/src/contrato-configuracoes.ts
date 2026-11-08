/** Configurações: situação das integrações, sem valor nenhum (só nome e se está preenchida). Conferido em `conformidade.ts`. */

export interface VariavelDeIntegracao {
  nome: string;
  descricao: string;
  obrigatoria: boolean;
  segredo: boolean;
  preenchida: boolean;
}

export interface IntegracaoVista {
  id: string;
  nome: string;
  para_que: string;
  situacao: 'em_uso' | 'desenvolvimento' | 'incompleta' | 'aguardando_adaptador' | 'nao_configurada';
  o_que_falta?: string | null;
  variaveis: VariavelDeIntegracao[];
  pendentes_no_outbox: number;
}
