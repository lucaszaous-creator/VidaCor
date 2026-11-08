import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { LaudoDoPortal } from '@vidacor/api-client';
import { Aviso, Botao, Carregando, Chip, formatarDataHora, SemConexao, Vazio } from '@vidacor/design-system';
import { consultaPortalLaudos } from '../../consultas';
import { useDependencias } from '../../contexto';
import { erroParaTela } from '../../erros';
import { FUSO_DA_CLINICA } from '../agenda/grade';

/** Erro do portal sem jargão: sessão de paciente separada da equipe. */
export function ErroDoPortal({ erro, aoTentarDeNovo }: { erro: unknown; aoTentarDeNovo: () => void }) {
  const e = erroParaTela(erro);
  const tentar = (
    <Botao pequeno onPress={aoTentarDeNovo}>
      Tentar de novo
    </Botao>
  );
  if (e.tipo === 'sem-sessao') {
    return (
      <Aviso nivel="atencao" titulo="Entre no portal para ver seus resultados.">
        Sua sessão terminou ou ainda não começou. Nada foi mostrado.
      </Aviso>
    );
  }
  if (e.tipo === 'sem-conexao') {
    return <SemConexao acao={tentar}>Seus resultados não carregaram. Confira a internet e tente de novo.</SemConexao>;
  }
  return (
    <Aviso nivel="bloqueante" codigoDeSuporte={e.codigoDeSuporte} acoes={tentar}>
      {e.mensagem}
    </Aviso>
  );
}

/** Marca de retificação (FC-LAU-05): o paciente vê só a versão vigente, sabendo que ela corrigiu outra. */
export function MarcaDeRetificacao({ laudo }: { laudo: Pick<LaudoDoPortal, 'retificado'> }) {
  return laudo.retificado ? <Chip cor="ambar" texto="Retificado" /> : <Chip cor="verde" texto="Assinado" />;
}

/** Portal do paciente: laudos publicados (só assinados e liberados), com a marca de retificação. */
export function TelaPortal() {
  const { api } = useDependencias();
  const lista = useQuery(consultaPortalLaudos(api));
  return (
    <>
      <h1 style={{ fontSize: 26, fontWeight: 650 }}>Seus resultados</h1>
      <section className="card" aria-label="Laudos publicados">
        {lista.isPending && <Carregando rotulo="Carregando seus resultados" />}
        {lista.isError && <ErroDoPortal erro={lista.error} aoTentarDeNovo={() => void lista.refetch()} />}
        {lista.data?.length === 0 && <Vazio titulo="Nenhum laudo publicado ainda" detalhe="O laudo aparece aqui depois de assinado pelo médico." />}
        {lista.data !== undefined && lista.data.length > 0 && (
          <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
            {lista.data.map((l) => (
              <li key={l.id} className="li">
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className="t">{l.exame}</div>
                  <div className="s">
                    Publicado em {formatarDataHora(l.publicado_em, FUSO_DA_CLINICA)}
                  </div>
                </div>
                <MarcaDeRetificacao laudo={l} />
                <Link to="/portal/laudos/$laudoId" params={{ laudoId: l.id }} className="btn btn-sm" aria-label={`Ler o laudo de ${l.exame}`}>
                  Ler laudo
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
      <p className="t13 mute" style={{ margin: 0 }}>
        Laudo novo aparece aqui depois de assinado pelo médico.
      </p>
    </>
  );
}
