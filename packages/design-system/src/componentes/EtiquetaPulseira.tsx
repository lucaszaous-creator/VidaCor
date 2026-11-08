export interface PropsDaEtiquetaPulseira {
  nome: string;
  /** Já formatado (dd/mm/aaaa). */
  nascimento: string;
  /** `ETQ-AAAA-NNNNNN`. */
  codigo: string;
  /** `ACC-AAAA-NNNNNN`, quando o atendimento já tem accession. */
  accession?: string;
  situacao: 'valida' | 'invalidada';
  /** Motivo da invalidação, quando invalidada. */
  motivo?: string | null;
  /** Código da etiqueta que substituiu esta. */
  substituidaPor?: string | null;
}

/**
 * Pré-visualização da pulseira (Etiquetas.dc.html). Por regra (docs/20) a etiqueta leva só nome,
 * nascimento e códigos: nunca diagnóstico, exame ou convênio. O código de barras é desenhado pela
 * impressora a partir do código; aqui o código aparece em texto, que é o que o leitor confere.
 */
export function EtiquetaPulseira({ nome, nascimento, codigo, accession, situacao, motivo, substituidaPor }: PropsDaEtiquetaPulseira) {
  const invalidada = situacao === 'invalidada';
  return (
    <figure
      className={invalidada ? 'etq etq-inv' : 'etq'}
      aria-label={`Pulseira ${codigo}, ${invalidada ? 'invalidada' : 'válida'}`}
      style={{ margin: 0 }}
    >
      <div className="etq-faixa">
        <div style={{ minWidth: 0 }}>
          <div className="etq-nome">{nome.toLocaleUpperCase('pt-BR')}</div>
          <div className="etq-nasc num">Nasc. {nascimento}</div>
        </div>
        <div className="etq-cods num">
          <div>{codigo}</div>
          {accession !== undefined && <div>{accession}</div>}
        </div>
      </div>
      <figcaption className="t13" style={{ marginTop: 6, color: invalidada ? 'var(--rose)' : 'var(--mute)' }}>
        {invalidada
          ? `Invalidada${motivo === undefined || motivo === null || motivo === '' ? '' : `: ${motivo}`}.${
              substituidaPor === undefined || substituidaPor === null ? '' : ` Substituída por ${substituidaPor}.`
            } Se for bipada, o sistema bloqueia.`
          : 'Válida. Só nome, nascimento e códigos.'}
      </figcaption>
    </figure>
  );
}
