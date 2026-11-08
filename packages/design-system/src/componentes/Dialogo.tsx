import type { ReactNode } from 'react';
import { Dialog, Heading, Modal, ModalOverlay } from 'react-aria-components';
import { Botao } from './Botao';

export interface PacienteDoDialogo {
  nome: string;
  /** Já formatado para leitura (dd/mm/aaaa). */
  nascimento: string;
}

export interface PropsDoDialogo {
  aberto: boolean;
  /** Diz o quê e com quem, repetindo o verbo do botão; nunca "Tem certeza?" (R3). */
  titulo: string;
  /** Os dois identificadores do paciente, sempre visíveis na confirmação (R2, R3). */
  paciente: PacienteDoDialogo;
  /** Detalhe do ato (exame, sala, horário) e a consequência. */
  children?: ReactNode;
  rotuloConfirmar: string;
  rotuloVoltar?: string;
  aoConfirmar: () => void;
  aoFechar: () => void;
  /** Enquanto o servidor não responde: não fecha nem reenvia. */
  ocupado?: boolean;
}

/**
 * Confirmação de ato irreversível (R3). Foco preso dentro, Esc volta, foco devolvido a quem abriu.
 * O foco começa em "Voltar", a opção que não muda nada.
 */
export function Dialogo({
  aberto,
  titulo,
  paciente,
  children,
  rotuloConfirmar,
  rotuloVoltar = 'Voltar',
  aoConfirmar,
  aoFechar,
  ocupado = false,
}: PropsDoDialogo) {
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
        <Dialog role="alertdialog">
          <Heading slot="title">{titulo}</Heading>
          <p className="dlg-pac" style={{ margin: 0 }}>
            <b>{paciente.nome}</b>, nascimento <b className="num">{paciente.nascimento}</b>
          </p>
          {children}
          <div className="dlg-r">
            <Botao pequeno onPress={aoFechar} isDisabled={ocupado} autoFocus>
              {rotuloVoltar}
            </Botao>
            <Botao pequeno variante="principal" onPress={aoConfirmar} ocupado={ocupado}>
              {rotuloConfirmar}
            </Botao>
          </div>
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}
