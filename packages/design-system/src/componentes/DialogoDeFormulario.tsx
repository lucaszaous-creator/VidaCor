import type { ReactNode } from 'react';
import { Dialog, Heading, Modal, ModalOverlay } from 'react-aria-components';
import { Botao } from './Botao';
import type { PacienteDoDialogo } from './Dialogo';

export interface PropsDoDialogoDeFormulario {
  aberto: boolean;
  titulo: string;
  /** Quando o ato é sobre um paciente, os dois identificadores aparecem no topo (R2). */
  paciente?: PacienteDoDialogo;
  children: ReactNode;
  rotuloConfirmar: string;
  rotuloVoltar?: string;
  aoConfirmar: () => void;
  aoFechar: () => void;
  ocupado?: boolean;
  /** Ato irreversível (invalida etiqueta, assina, estorna): papel `alertdialog` (R3). */
  irreversivel?: boolean;
}

/**
 * Diálogo com campos (motivo, PIN, justificativa). Foco preso dentro, Esc volta, foco devolvido a
 * quem abriu. O envio é pelo botão de confirmar; Enter num campo de uma linha também envia.
 */
export function DialogoDeFormulario({
  aberto,
  titulo,
  paciente,
  children,
  rotuloConfirmar,
  rotuloVoltar = 'Voltar',
  aoConfirmar,
  aoFechar,
  ocupado = false,
  irreversivel = false,
}: PropsDoDialogoDeFormulario) {
  return (
    <ModalOverlay
      isOpen={aberto}
      onOpenChange={(abertoAgora) => {
        if (!abertoAgora && !ocupado) {
          aoFechar();
        }
      }}
      isDismissable={false}
      isKeyboardDismissDisabled={ocupado}
      className="dlg-fundo"
    >
      <Modal className="dlg">
        <Dialog role={irreversivel ? 'alertdialog' : 'dialog'}>
          <form
            className="col"
            style={{ gap: 12 }}
            noValidate
            onSubmit={(ev) => {
              ev.preventDefault();
              if (!ocupado) {
                aoConfirmar();
              }
            }}
          >
            <Heading slot="title">{titulo}</Heading>
            {paciente !== undefined && (
              <p className="dlg-pac" style={{ margin: 0 }}>
                <b>{paciente.nome}</b>, nascimento <b className="num">{paciente.nascimento}</b>
              </p>
            )}
            {children}
            <div className="dlg-r">
              <Botao pequeno onPress={aoFechar} isDisabled={ocupado}>
                {rotuloVoltar}
              </Botao>
              <Botao pequeno variante="principal" type="submit" ocupado={ocupado}>
                {rotuloConfirmar}
              </Botao>
            </div>
          </form>
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}
