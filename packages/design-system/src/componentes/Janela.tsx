import type { ReactNode } from 'react';
import { Dialog, Heading, Modal, ModalOverlay } from 'react-aria-components';

export interface PropsDaJanela {
  aberto: boolean;
  titulo: string;
  /** Título só para leitor de tela (a janela mostra o próprio conteúdo no topo). */
  tituloOculto?: boolean;
  aoFechar: () => void;
  children: ReactNode;
}

/**
 * Janela sobreposta para ferramentas (busca global): foco preso dentro, Esc e clique fora fecham, foco
 * devolvido a quem abriu. Não serve para confirmar ato clínico: para isso existe o `Dialogo` (R3).
 */
export function Janela({ aberto, titulo, tituloOculto = false, aoFechar, children }: PropsDaJanela) {
  return (
    <ModalOverlay
      isOpen={aberto}
      onOpenChange={(abertoAgora) => {
        if (!abertoAgora) aoFechar();
      }}
      isDismissable
      className="dlg-fundo"
    >
      <Modal className="dlg" style={{ width: 'min(640px, calc(100vw - 32px))' }}>
        <Dialog aria-label={tituloOculto ? titulo : undefined}>
          {!tituloOculto && <Heading slot="title">{titulo}</Heading>}
          {children}
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}
