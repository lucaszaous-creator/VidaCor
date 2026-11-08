import { useMutation } from '@tanstack/react-query';
import type { DocumentoBaixado } from '@vidacor/api-client';
import { Botao, type VarianteDoBotao } from '@vidacor/design-system';
import type { ReactNode } from 'react';
import { useDependencias } from '../contexto';
import { ErroDoAto } from './ErroDoAto';

/**
 * Entrega o PDF ao navegador como arquivo baixado. O conteúdo vive só no endereço temporário do
 * próprio arquivo (liberado logo depois); nada vai para armazenamento do navegador (SEC-TEL).
 */
export function salvarNoNavegador(doc: DocumentoBaixado): void {
  const url = URL.createObjectURL(doc.conteudo);
  const a = document.createElement('a');
  a.href = url;
  a.download = doc.nomeDoArquivo;
  a.rel = 'noopener';
  a.hidden = true;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 10_000);
}

export interface PropsDoBaixarDocumento {
  /** Texto do botão ("Baixar PDF"). */
  rotulo: string;
  /** O ato, no infinitivo, para a mensagem de erro ("baixar o PDF do laudo"). */
  ato: string;
  baixar: () => Promise<DocumentoBaixado>;
  variante?: VarianteDoBotao;
  /** Mostra o erro do jeito da tela (o portal do paciente tem o seu). Padrão: `ErroDoAto` da equipe. */
  mostrarErro?: (erro: unknown, tentarDeNovo: () => void) => ReactNode;
  /** Para o paciente: confirma só o arquivo, sem o código da emissão e o hash. */
  simples?: boolean;
}

/**
 * Botão que pede o PDF ao núcleo e o entrega como arquivo. O servidor decide se pode (a mesma
 * autorização da leitura) e devolve o SHA-256 dos bytes, que a tela mostra como veio.
 */
export function BaixarDocumento({ rotulo, ato, baixar, variante = 'padrao', mostrarErro, simples = false }: PropsDoBaixarDocumento) {
  const { salvarArquivo } = useDependencias();
  const m = useMutation({
    mutationFn: baixar,
    onSuccess: (doc) => {
      (salvarArquivo ?? salvarNoNavegador)(doc);
    },
  });
  return (
    <div className="col" style={{ gap: 6, minWidth: 0 }}>
      <div className="row">
        <Botao
          variante={variante}
          ocupado={m.isPending}
          onPress={() => {
            m.mutate();
          }}
        >
          {m.isPending ? 'Gerando o PDF…' : rotulo}
        </Botao>
      </div>
      {m.isSuccess && (
        <p className="t13 mute num" role="status" style={{ margin: 0, overflowWrap: 'anywhere' }}>
          PDF baixado: {m.data.nomeDoArquivo}.
          {!simples && ` Emissão ${m.data.codigoDaEmissao} registrada, SHA-256 ${m.data.sha256}.`}
        </p>
      )}
      {m.isError &&
        (mostrarErro === undefined ? (
          <ErroDoAto
            erro={m.error}
            ato={ato}
            aoTentarDeNovo={() => {
              m.mutate();
            }}
          />
        ) : (
          mostrarErro(m.error, () => {
            m.mutate();
          })
        ))}
    </div>
  );
}
