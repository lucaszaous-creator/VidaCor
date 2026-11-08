import type { CSSProperties } from 'react';

/** Iniciais para o avatar: primeira letra do primeiro e do último nome. */
export function iniciais(nome: string): string {
  const partes = nome.trim().split(/\s+/).filter(Boolean);
  const primeira = partes[0]?.charAt(0) ?? '';
  const ultima = partes.length > 1 ? (partes[partes.length - 1]?.charAt(0) ?? '') : '';
  return (primeira + ultima).toLocaleUpperCase('pt-BR');
}

export interface PropsDoAvatar {
  nome: string;
  tamanho?: 'pequeno' | 'medio' | 'grande';
  /** Fundo e cor do texto (padrão: marinho com branco, como no menu lateral). */
  cores?: { fundo: string; texto: string };
}

const CLASSE = { pequeno: 'av av-sm', medio: 'av', grande: 'av av-lg' } as const;

/** Avatar com iniciais. É decorativo: o nome sempre aparece escrito ao lado. */
export function Avatar({ nome, tamanho = 'medio', cores }: PropsDoAvatar) {
  const estilo: CSSProperties = {
    background: cores?.fundo ?? 'var(--navy)',
    color: cores?.texto ?? '#fff',
  };
  return (
    <span className={CLASSE[tamanho]} style={estilo} aria-hidden="true">
      {iniciais(nome)}
    </span>
  );
}
