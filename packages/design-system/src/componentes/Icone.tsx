/**
 * Conjunto único de ícones do sistema: SVG de traço, 24 px de grade, copiados dos mockups
 * (Sidebar, Agenda, Estados, Barra-Paciente). Nada de emoji nem caractere usado como ícone.
 */
const CAMINHOS = {
  inicio: 'M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z',
  agenda: 'M4 5h16v16H4zM4 10h16M9 3v4M15 3v4',
  recepcao:
    'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM22 21v-2a4 4 0 0 0-3-3.9M16 3.1a4 4 0 0 1 0 7.8',
  busca: 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-3.5-3.5',
  anterior: 'M15 6l-6 6 6 6',
  proximo: 'M9 6l6 6-6 6',
  abaixo: 'M6 9l6 6 6-6',
  fechar: 'M6 6l12 12M18 6L6 18',
  recolher: 'M3 7a3 3 0 0 1 3-3h12a3 3 0 0 1 3 3v10a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3zM9 4v16M16 10l-2 2 2 2',
  expandir: 'M3 7a3 3 0 0 1 3-3h12a3 3 0 0 1 3 3v10a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3zM9 4v16M14 10l2 2-2 2',
  maisOpcoes: 'M5 12h.01M12 12h.01M19 12h.01',
  confere: 'M5 12l5 5 9-10',
  alerta:
    'M12 8v5M12 16.5h.01M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z',
  erro: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v6M12 16.5h.01',
  info: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 7.5h.01',
  semConexao: 'M2 8.8a15 15 0 0 1 20 0M5 12.6a10 10 0 0 1 14 0M8.5 16.4a5 5 0 0 1 7 0M12 20h.01M3 3l18 18',
  cadeado: 'M7 11h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2zM8 11V7a4 4 0 0 1 8 0v4',
  sair: 'M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 17l-5-5 5-5M5 12h11',
  imagem: 'M12 3l9 5-9 5-9-5zM3 13l9 5 9-5M3 17l9 5 9-5',
  // Módulos da Fase 1, copiados de Sidebar.dc.html (mesmo traço e grade).
  agendar: 'M4 5h16v16H4zM4 10h16M9 3v4M15 3v4M8 14h3v3H8z',
  sala: 'M3 4h18v12H3zM8 20h8M12 16v4M7 12l3-4 2 3 2-2 3 3',
  laudos: 'M14 3H6v18h12V7zM14 3v4h4M9 13h6M9 17h4',
  caixa: 'M3 8h18v12H3zM7 8V5h10v3M12 12v4M10 14h4',
  etiquetas: 'M20 12l-8 8-9-9V3h8zM7.5 7.5h.01',
  contingencia: 'M12 9v4M12 17h.01M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z',
  triagem: 'M9 3h6v4H9zM6 5h3M15 5h3v16H6V5h3M9 12h6M9 16h4',
  autorizacao: 'M12 21s-7-4.4-7-10V5l7-3 7 3v6c0 5.6-7 10-7 10zM9 12l2 2 4-4',
  estorno: 'M9 14L4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3',
  imprimir: 'M6 9V3h12v6M6 18H4a1 1 0 0 1-1-1v-6a1 1 0 0 1 1-1h16a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-2M6 14h12v7H6z',
  copiar: 'M9 9h11v11H9zM5 15H4V4h11v1',
  leitor: 'M4 6v12M7 6v12M11 6v12M14 6v12M18 6v12M20 6v12',
  letraMaior: 'M4 18L9 6l5 12M6 14h6M18 9v6M15 12h6',
  letraMenor: 'M4 18L9 6l5 12M6 14h6M15 12h6',
} as const;

export type NomeDoIcone = keyof typeof CAMINHOS;

export interface PropsDoIcone {
  nome: NomeDoIcone;
  tamanho?: number;
  traco?: number;
  className?: string;
}

export function Icone({ nome, tamanho = 18, traco = 2, className }: PropsDoIcone) {
  return (
    <svg
      className={className}
      width={tamanho}
      height={tamanho}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={traco}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={CAMINHOS[nome]} />
    </svg>
  );
}
