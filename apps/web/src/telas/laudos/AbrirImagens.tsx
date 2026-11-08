import { useMutation } from '@tanstack/react-query';
import { Botao } from '@vidacor/design-system';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';

/**
 * Abrir as imagens no visualizador (Visualizador.dc.html; ADR-0006). O núcleo autoriza (estudo
 * conciliado e vínculo de cuidado) e registra cada abertura; só então a tela mostra o endereço.
 */
export function AbrirImagens({ estudoId }: { estudoId: string }) {
  const { api } = useDependencias();
  const abrir = useMutation({ mutationFn: () => api.estudos.visualizacao(estudoId) });
  const r = abrir.data;
  if (r !== undefined) {
    return r.url === null || r.url === undefined ? (
      <span className="t13 mute" role="status">
        Visualizador ainda não instalado nesta unidade. O acesso às {r.imagens} imagens foi registrado.
      </span>
    ) : (
      <a className="btn btn-sm" href={r.url} target="_blank" rel="noopener noreferrer">
        Abrir no visualizador ({r.imagens} imagens)
      </a>
    );
  }
  return (
    <span className="row" style={{ gap: 8 }}>
      <Botao
        pequeno
        ocupado={abrir.isPending}
        onPress={() => {
          abrir.mutate();
        }}
      >
        Abrir imagens
      </Botao>
      {abrir.isError && <ErroDoAto erro={abrir.error} ato="abrir as imagens" />}
    </span>
  );
}
