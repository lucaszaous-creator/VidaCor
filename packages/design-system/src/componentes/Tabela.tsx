import type { ReactNode } from 'react';
import { Cell, Column, Row, Table, TableBody, TableHeader } from 'react-aria-components';

export interface ColunaDaTabela<T> {
  id: string;
  rotulo: string;
  /** Coluna que nomeia a linha para o leitor de tela (normalmente o paciente). */
  cabecalhoDaLinha?: boolean;
  /** `nw`: valor curto que não quebra (data, hora, valor). */
  semQuebra?: boolean;
  direita?: boolean;
  celula: (linha: T) => ReactNode;
}

export interface PropsDaTabela<T extends { id: string }> {
  rotulo: string;
  colunas: ColunaDaTabela<T>[];
  linhas: T[];
  /** Texto quando não há linhas (estado vazio dentro da tabela). */
  vazio?: string;
  aoAbrir?: (linha: T) => void;
}

/**
 * Tabela do vc10 (`tb sticky`): ocupa 100%, quebra texto nas células, cabeçalho fixo ao rolar e
 * sem `min-width` (nada de rolagem lateral). Navegação por teclado do React Aria.
 */
export function Tabela<T extends { id: string }>({ rotulo, colunas, linhas, vazio, aoAbrir }: PropsDaTabela<T>) {
  const classeDa = (c: ColunaDaTabela<T>): string =>
    [c.semQuebra === true ? 'nw' : '', c.direita === true ? 'r' : ''].filter(Boolean).join(' ');
  return (
    <Table aria-label={rotulo} className="tb sticky" onRowAction={aoAbrir === undefined ? undefined : (chave) => {
      const linha = linhas.find((l) => l.id === chave);
      if (linha !== undefined) {
        aoAbrir(linha);
      }
    }}>
      <TableHeader>
        {colunas.map((c) => (
          <Column key={c.id} id={c.id} isRowHeader={c.cabecalhoDaLinha ?? false} className={classeDa(c)}>
            {c.rotulo}
          </Column>
        ))}
      </TableHeader>
      <TableBody items={linhas} renderEmptyState={() => <span className="mute">{vazio ?? 'Nada para mostrar.'}</span>}>
        {(linha) => (
          <Row id={linha.id}>
            {colunas.map((c) => (
              <Cell key={c.id} className={classeDa(c)}>
                {c.celula(linha)}
              </Cell>
            ))}
          </Row>
        )}
      </TableBody>
    </Table>
  );
}
