import { useQuery } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import { ErroDaApi } from '@vidacor/api-client';
import { Aviso, Carregando } from '@vidacor/design-system';
import { consultaLink } from '../../consultas';
import { useDependencias } from '../../contexto';
import { LeituraDoLaudo } from './TelaLaudoDoPortal';
import { ErroDoPortal } from './TelaPortal';

const rota = getRouteApi('/portal/link/$token');

/** Laudo compartilhado por link com médico externo (FC-ACE-08): expirado, nada é exibido. */
export function TelaLinkCompartilhado() {
  const { api } = useDependencias();
  const { token } = rota.useParams();
  const consulta = useQuery({ ...consultaLink(api, token), retry: false });
  const erro = consulta.error;
  const expirado = erro instanceof ErroDaApi && erro.status === 410;
  const inexistente = erro instanceof ErroDaApi && erro.status === 404;
  return (
    <>
      {consulta.isPending && <Carregando rotulo="Carregando o laudo compartilhado" />}
      {expirado && (
        <Aviso nivel="atencao" titulo="Este link expirou.">
          O laudo não é mais exibido por este link. Peça ao paciente que compartilhe um link novo pelo portal.
        </Aviso>
      )}
      {inexistente && (
        <Aviso nivel="informativo" titulo="Link não encontrado.">
          Confira se o endereço foi copiado inteiro ou peça ao paciente um link novo.
        </Aviso>
      )}
      {erro !== null && !expirado && !inexistente && <ErroDoPortal erro={erro} aoTentarDeNovo={() => void consulta.refetch()} />}
      {consulta.data !== undefined && <LeituraDoLaudo laudo={consulta.data} />}
    </>
  );
}
