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
