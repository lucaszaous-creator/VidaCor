import { useQuery } from '@tanstack/react-query';
import { getRouteApi, Link } from '@tanstack/react-router';
import { ErroDaApi, type LaudoDoPortal } from '@vidacor/api-client';
import { Aviso, Carregando, formatarDataHora } from '@vidacor/design-system';
import { consultaPortalLaudo } from '../../consultas';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { ErroDoPortal, MarcaDeRetificacao } from './TelaPortal';

const rota = getRouteApi('/portal/laudos/$laudoId');

/** Leitura do laudo publicado, sempre como texto (nunca HTML). */
export function LeituraDoLaudo({ laudo }: { laudo: LaudoDoPortal }) {
  return (
    <article className="card col" style={{ gap: 12 }} aria-label={`Laudo de ${laudo.exame.nome}`}>
      <div className="row wrap" style={{ gap: 10 }}>
        <h1 style={{ fontSize: 22, fontWeight: 650 }}>{laudo.exame.nome}</h1>
        <MarcaDeRetificacao laudo={laudo} />
      </div>
      <div className="t14 mute">
        {laudo.medico}, assinado em {formatarDataHora(laudo.assinado_em, FUSO_DA_CLINICA)}. Laudo {laudo.codigo}.
      </div>
      {laudo.retificado && (
        <Aviso nivel="informativo" titulo="Este laudo foi corrigido pelo médico.">
          Você está vendo a versão {laudo.versao}, que substitui a anterior.
        </Aviso>
      )}
      <pre className="texto-laudo">{laudo.texto}</pre>
    </article>
  );
}

/**
 * Laudo no portal. "Não encontrado" tem a mesma mensagem para laudo de outro paciente e laudo que não
 * existe (FC-ACE-02): a tela não revela nada além disso.
 */
export function TelaLaudoDoPortal() {
  const { api } = useDependencias();
  const { laudoId } = rota.useParams();
  const consulta = useQuery(consultaPortalLaudo(api, laudoId));
  const naoEncontrado = consulta.error instanceof ErroDaApi && consulta.error.status === 404;
  return (
    <>
      <div>
        <Link to="/portal" className="t14 b">
          Voltar aos resultados
        </Link>
      </div>
      {consulta.isPending && <Carregando rotulo="Carregando o laudo" />}
      {naoEncontrado ? (
        <Aviso nivel="informativo" titulo="Laudo não encontrado.">
          Não encontramos este laudo na sua conta. Confira a lista dos seus resultados.
        </Aviso>
      ) : (
        consulta.isError && <ErroDoPortal erro={consulta.error} aoTentarDeNovo={() => void consulta.refetch()} />
      )}
      {consulta.data !== undefined && <LeituraDoLaudo laudo={consulta.data} />}
    </>
  );
}
