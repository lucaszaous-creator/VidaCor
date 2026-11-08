/** Menu de contexto vindo do servidor (docs/19 § 8). Conferido contra o OpenAPI em `conformidade.ts`. */

export type ObjetoDoMenu = 'agendamento' | 'atendimento' | 'laudo';

export interface AcaoDoMenuDoServidor {
  id: string;
  rotulo: string;
  estado: 'disponivel' | 'indisponivel';
  motivo?: string | null;
  perigo: boolean;
  /** Tela que faz a ação (só identificadores técnicos); sem destino, a tela trata (ex.: copiar). */
  destino?: string | null;
}

export interface MenuDoObjeto {
  cabecalho: { titulo: string; subtitulo: string };
  acoes: AcaoDoMenuDoServidor[];
}
