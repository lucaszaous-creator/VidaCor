import { useEffect, useState } from 'react';
import { Aviso } from '@vidacor/design-system';
import { CANAL_DE_REAUTENTICACAO, MENSAGEM_REAUTENTICADO } from '../reautenticacao';

/**
 * Volta da janela de reautenticação (DEC-2026-11-08-54). Avisa a tela de origem pelo canal da mesma
 * origem (a mensagem não leva dado nenhum) e fecha a janela. Se o navegador não deixar fechar, diz
 * o que fazer.
 */
export function TelaReautenticado() {
  const [fechou, setFechou] = useState(true);
  useEffect(() => {
    if (typeof BroadcastChannel !== 'undefined') {
      const canal = new BroadcastChannel(CANAL_DE_REAUTENTICACAO);
      canal.postMessage({ tipo: MENSAGEM_REAUTENTICADO });
      canal.close();
    }
    window.close();
    const t = window.setTimeout(() => { setFechou(false); }, 300);
    return () => { window.clearTimeout(t); };
  }, []);
  return (
    <main style={{ padding: 24, maxWidth: 480 }}>
      <h1>Identidade confirmada</h1>
      {!fechou && (
        <Aviso nivel="informativo" titulo="Pode fechar esta janela.">
          A tela de onde você veio já recebeu a confirmação e repete o pedido sozinha.
        </Aviso>
      )}
    </main>
  );
}
