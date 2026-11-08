import { Vazio } from '@vidacor/design-system';
import { CabecalhoDaTela } from '../../componentes/Bloco';
import { FilaDeLaudos } from './FilaDeLaudos';

/** Laudar: a fila à esquerda e o laudo escolhido à direita. */
export function TelaFilaDeLaudos() {
  return (
    <>
      <CabecalhoDaTela titulo="Laudar" />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: '300px minmax(0, 1fr)', gap: 16 }}>
        <FilaDeLaudos />
        <section className="card pane" aria-label="Laudo">
          <Vazio titulo="Escolha um laudo da fila" detalhe="O rascunho, a assinatura e as versões aparecem aqui." />
        </section>
      </div>
    </>
  );
}
