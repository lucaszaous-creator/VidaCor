import axe from 'axe-core';

/**
 * Roda o axe (WCAG 2.2 A e AA) e devolve as violações em texto, para o teste falhar mostrando o quê.
 * `color-contrast` fica de fora porque o jsdom não calcula estilo; o contraste é dos tokens (docs/05)
 * e da conferência no navegador real.
 */
export async function violacoesDeAcessibilidade(elemento: Element): Promise<string[]> {
  const resultado = await axe.run(elemento, {
    runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'] },
    rules: { 'color-contrast': { enabled: false } },
  });
  return resultado.violations.map((v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(' ')).join(', ')})`);
}
