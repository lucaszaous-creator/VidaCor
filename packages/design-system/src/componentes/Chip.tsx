export type CorDoChip = 'verde' | 'ambar' | 'rosa' | 'azul' | 'violeta' | 'verdeAgua' | 'marinho' | 'cinza';

const CLASSE: Record<CorDoChip, string> = {
  verde: 'chip c-green',
  ambar: 'chip c-amber',
  rosa: 'chip c-rose',
  azul: 'chip c-blue',
  violeta: 'chip c-violet',
  verdeAgua: 'chip c-teal',
  marinho: 'chip c-navy',
  cinza: 'chip',
};

export interface PropsDoChip {
  cor: CorDoChip;
  /** Status é cor MAIS texto, nunca só cor (docs/05, regra 5). */
  texto: string;
  /** Ponto colorido antes do texto (`.d`), como no mockup de Login. */
  ponto?: boolean;
}

/** Chip só para situação ou status, no máximo um por linha de lista (docs/05). */
export function Chip({ cor, texto, ponto = false }: PropsDoChip) {
  return (
    <span className={CLASSE[cor]}>
      {ponto && <span className="d" aria-hidden="true" />}
      {texto}
    </span>
  );
}
