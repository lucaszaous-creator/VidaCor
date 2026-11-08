import { useQuery } from '@tanstack/react-query';
import { Botao, Carregando, Chip, ErroTela, formatarHora, SemConexao } from '@vidacor/design-system';
import { consultaAtendimento } from '../../consultas';
import { useDependencias } from '../../contexto';
import { erroParaTela } from '../../erros';
import { FUSO_DA_CLINICA } from './grade';
import { aparenciaDoAtendimento } from './situacoes';

/** Atendimento e situação do estudo, sempre lidos do servidor (atualizados pelo SSE). */
export function ResumoDoAtendimento({ atendimentoId }: { atendimentoId: string }) {
  const { api } = useDependencias();
  const consulta = useQuery(consultaAtendimento(api, atendimentoId));

  if (consulta.isPending) {
    return <Carregando rotulo="Carregando o atendimento" linhas={1} />;
  }
  if (consulta.isError) {
    const e = erroParaTela(consulta.error);
    const tentar = (
      <Botao pequeno onPress={() => void consulta.refetch()}>
        Tentar de novo
      </Botao>
    );
    return e.tipo === 'sem-conexao' ? (
      <SemConexao acao={tentar}>O atendimento não carregou. Nada foi alterado.</SemConexao>
    ) : (
      <ErroTela mensagem={e.mensagem} codigoDeSuporte={e.codigoDeSuporte} acao={tentar} />
    );
  }

  const a = consulta.data;
  const ap = aparenciaDoAtendimento(a.situacao);
  return (
    <div className="col" style={{ gap: 10, borderTop: '1px solid var(--line)', paddingTop: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h3 className="t14 b">Atendimento</h3>
        <Chip cor={ap.cor} texto={ap.texto} />
      </div>
      <dl className="kv" style={{ margin: 0, gridTemplateColumns: '96px minmax(0, 1fr)' }}>
        <dt>Código</dt>
        <dd className="num">{a.codigo}</dd>
        <dt>Accession</dt>
        <dd className="num">{a.accession}</dd>
        <dt>Imagens</dt>
        <dd>
          {a.estudo === null || a.estudo === undefined
            ? 'Ainda não chegaram do aparelho.'
            : `${String(a.estudo.imagens)} ${a.estudo.imagens === 1 ? 'imagem recebida' : 'imagens recebidas'} às ${formatarHora(a.estudo.recebido_em, FUSO_DA_CLINICA)}.`}
        </dd>
      </dl>
    </div>
  );
}
