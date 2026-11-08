import { Botao } from '@vidacor/design-system';
import { useState } from 'react';
import { useDependencias } from '../../contexto';
import { hojeNaClinica } from './grade';

/**
 * Pede a confirmação por WhatsApp dos agendamentos do dia mostrado (só dias que ainda não passaram).
 * O servidor enfileira um aviso por agendamento; repetir não manda de novo.
 */
export function ConfirmarPorWhatsapp({ data }: { data: string }) {
  const { api } = useDependencias();
  const [estado, setEstado] = useState<{ data: string; texto: string } | null>(null);
  const [enviando, setEnviando] = useState(false);
  if (data < hojeNaClinica()) return null;
  return (
    <>
      <Botao
        pequeno
        ocupado={enviando}
        onPress={() => {
          setEnviando(true);
          api.comunicacao
            .lembretes(data)
            .then(
              (r) => {
                setEstado({ data, texto: r.enfileirados === 0 ? 'Nada novo a confirmar neste dia.' : `${String(r.enfileirados)} confirmação(ões) a caminho.` });
              },
              () => {
                setEstado({ data, texto: 'As confirmações não foram pedidas. Tente de novo.' });
              },
            )
            .finally(() => {
              setEnviando(false);
            });
        }}
      >
        Confirmar por WhatsApp
      </Botao>
      {estado?.data === data && (
        <span className="t13" role="status">
          {estado.texto}
        </span>
      )}
    </>
  );
}
