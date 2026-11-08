import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { AchadoEstruturado, AchadosDoLaudo, Laudo } from '@vidacor/api-client';
import { Aviso, Botao, Chip, Tabela, type ColunaDaTabela } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { chaves } from '../../consultas';
import { useDependencias } from '../../contexto';
import { FormularioDeAchado } from './FormularioDeAchado';
import { corDaCategoria, formatarMilimetros } from './opcoesDosAchados';

const COLUNAS: ColunaDaTabela<AchadoEstruturado>[] = [
  { id: 'rotulo', rotulo: 'Achado', cabecalhoDaLinha: true, semQuebra: true, celula: (a) => <b>{a.rotulo}</b> },
  {
    id: 'local',
    rotulo: 'Local e medida',
    celula: (a) => (
      <span>
        {a.localizacao}, {formatarMilimetros(a.maior_diametro_mm)}
      </span>
    ),
  },
  { id: 'pontos', rotulo: 'Pontos', semQuebra: true, direita: true, celula: (a) => <span className="num">{a.pontos ?? ''}</span> },
  {
    id: 'categoria',
    rotulo: 'Categoria',
    semQuebra: true,
    celula: (a) => <Chip cor={corDaCategoria(a.categoria)} texto={a.sistema === 'tirads' ? a.categoria : `BI-RADS ${a.categoria}`} />,
  },
];

function Detalhe({ a, editavel, laudoId }: { a: AchadoEstruturado; editavel: boolean; laudoId: string }) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const remover = useMutation({
    mutationFn: () => api.laudos.removerAchado(laudoId, a.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: chaves.achadosDoLaudo(laudoId) });
    },
  });
  return (
    <section className="col" style={{ gap: 6, padding: '10px 14px', borderRadius: 'var(--r3)', background: 'var(--gray-bg)' }} aria-label={`Resultado de ${a.rotulo}`}>
      <p className="t14" style={{ margin: 0 }}>
        <b>
          {a.rotulo}: {a.pontos === null || a.pontos === undefined ? '' : `${String(a.pontos)} pontos, `}
          {a.sistema === 'tirads' ? `${a.categoria} ${a.categoria_nome}` : `BI-RADS ${a.categoria}, ${a.categoria_nome}`}
        </b>
      </p>
      <p className="t14" style={{ margin: 0 }}>Conduta: {a.conduta}.</p>
      <p className="t13 mute" style={{ margin: 0 }}>
        Calculado pelo servidor com a tabela {a.regra_versao}; a versão fica guardada no laudo.
      </p>
      {editavel && (
        <div className="row">
          <Botao
            pequeno
            ocupado={remover.isPending}
            onPress={() => {
              remover.mutate();
            }}
          >
            Remover {a.rotulo} do rascunho
          </Botao>
        </div>
      )}
      {remover.isError && <ErroDoAto erro={remover.error} ato={`remover ${a.rotulo}`} />}
    </section>
  );
}

export interface PropsDoPainelDeAchados {
  laudo: Laudo;
  /** Só no rascunho: leva o texto gerado dos achados para o editor (o médico revisa e edita). */
  aoLevarAoTexto?: (texto: string) => void;
}

/**
 * Achados estruturados do laudo (Laudo-Estruturado.dc.html, docs/22 E2): tabela dos achados com a
 * categoria devolvida pelo servidor, o resultado do selecionado e, no rascunho do médico atribuído,
 * o formulário de novo achado. A tela não calcula nada (G1, G5).
 */
export function PainelDeAchados({ laudo, aoLevarAoTexto }: PropsDoPainelDeAchados) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const versoes = laudo.versoes.length;
  const consulta = useQuery({
    queryKey: [...chaves.achadosDoLaudo(laudo.id), laudo.situacao, versoes],
    queryFn: () => api.laudos.achados(laudo.id),
  });
  const [selecionado, setSelecionado] = useState<string | null>(null);
  const [levado, setLevado] = useState(false);
  const lista: AchadosDoLaudo | undefined = consulta.data;

  if (lista === undefined) {
    return <EstadoDaConsulta oQue="os achados estruturados" carregando={consulta.isPending} erro={consulta.error} aoTentarDeNovo={() => void consulta.refetch()} />;
  }
  const atual = lista.achados.find((a) => a.id === selecionado) ?? lista.achados.at(-1);
  const regras = lista.regras_vigentes.map((r) => r.versao).join(' e ');

  return (
    <section className="col" style={{ gap: 10 }} aria-label="Achados estruturados">
      <div className="row wrap" style={{ gap: 10 }}>
        <h3 className="t15 b">Achados estruturados, versão {lista.versao_numero}</h3>
        {!lista.editavel && <Chip cor="cinza" texto="Somente leitura" />}
      </div>
      <p className="t13 mute" style={{ margin: 0 }}>
        Categoria e conduta são calculadas pelo servidor ({regras}). Medidas e descritores marcados pelo médico.
      </p>
      <Tabela
        rotulo="Achados do laudo"
        colunas={COLUNAS}
        linhas={lista.achados}
        vazio={lista.editavel ? 'Nenhum achado estruturado. Registre abaixo, se o exame tiver.' : 'Este laudo não tem achados estruturados.'}
        aoAbrir={(a) => {
          setSelecionado(a.id);
        }}
      />
      {atual !== undefined && <Detalhe key={atual.id} a={atual} editavel={lista.editavel} laudoId={laudo.id} />}
      {lista.editavel && aoLevarAoTexto !== undefined && lista.achados.length > 0 && (
        <div className="row wrap" style={{ gap: 10 }}>
          <Botao
            onPress={() => {
              aoLevarAoTexto(`${lista.texto_achados}\n\nConclusão:\n${lista.texto_conclusao}`);
              setLevado(true);
            }}
          >
            Levar os achados ao texto do laudo
          </Botao>
          {levado && (
            <span className="t13 mute" role="status">
              Texto dos achados acrescentado ao laudo. Revise antes de assinar.
            </span>
          )}
        </div>
      )}
      {lista.editavel && (
        <FormularioDeAchado
          laudoId={laudo.id}
          aoRegistrar={async (a) => {
            setSelecionado(a.id);
            await queryClient.invalidateQueries({ queryKey: chaves.achadosDoLaudo(laudo.id) });
          }}
        />
      )}
      {!lista.editavel && lista.achados.length > 0 && laudo.situacao === 'assinado' && (
        <Aviso nivel="informativo" titulo="Achados travados com a assinatura.">
          Para corrigir, crie uma retificação: os achados vão para a nova versão.
        </Aviso>
      )}
    </section>
  );
}
