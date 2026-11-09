import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import { ErroDaApi, type Laudo, type VersaoDoLaudo } from '@vidacor/api-client';
import {
  AreaDeTexto,
  Aviso,
  BarraDoPaciente,
  Botao,
  CaixaDeMarcar,
  Chip,
  formatarData,
  formatarDataHora,
  formatarHora,
  SemPermissao,
  type CorDoChip,
} from '@vidacor/design-system';
import { useState } from 'react';
import { BaixarDocumento } from '../../componentes/BaixarDocumento';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { chaves, consultaLaudo } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useRetomada, type Retomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { AcessoDeEmergencia, BotaoDeLiberacao, BotaoDePublicacao, CompartilharLaudo, DialogoDeAssinatura, DialogoDeRetificacao } from './AtosDoLaudo';
import { AbrirImagens } from './AbrirImagens';
import { FilaDeLaudos, SITUACAO_DO_LAUDO } from './FilaDeLaudos';
import { PainelDeAchados } from './PainelDeAchados';
import { PainelDeModelos } from './PainelDeModelos';
import { useRascunho } from './useRascunho';

const rota = getRouteApi('/app/laudos/$laudoId');

const VERSAO: Record<string, { texto: string; cor: CorDoChip }> = {
  rascunho: { texto: 'Rascunho', cor: 'azul' },
  assinada: { texto: 'Assinada', cor: 'verde' },
  substituida: { texto: 'Substituída', cor: 'cinza' },
};

function SituacaoDoRascunho({ r }: { r: ReturnType<typeof useRascunho> }) {
  if (r.situacao === 'salvando') return <span className="t13 mute" role="status">Salvando o rascunho…</span>;
  if (r.situacao === 'pendente') return <span className="t13 mute" role="status">Alterações ainda não salvas.</span>;
  if (r.situacao === 'sem-conexao' || r.situacao === 'erro') {
    return (
      <Aviso
        nivel="atencao"
        titulo="O rascunho não foi salvo."
        acoes={
          <Botao pequeno onPress={r.tentarDeNovo}>
            Salvar de novo
          </Botao>
        }
      >
        {r.situacao === 'sem-conexao' ? 'Sem conexão com o servidor.' : r.erro instanceof ErroDaApi ? r.erro.problema.detail : 'O servidor recusou.'} O texto continua aqui; nada
        foi perdido.
      </Aviso>
    );
  }
  return (
    <span className="t13 mute" role="status">
      {r.salvoEm === null ? 'Rascunho salvo no servidor.' : `Rascunho salvo às ${formatarHora(r.salvoEm.toISOString(), FUSO_DA_CLINICA)}.`}
    </span>
  );
}

function Versoes({ versoes, vigente }: { versoes: VersaoDoLaudo[]; vigente: number | null }) {
  return (
    <div className="col" style={{ gap: 8 }}>
      <h3 className="t15 b">Versões</h3>
      {[...versoes].reverse().map((v) => {
        const s = VERSAO[v.situacao] ?? { texto: 'Situação não reconhecida', cor: 'cinza' as const };
        return (
          <details key={v.numero} style={{ borderTop: '1px solid var(--line)', paddingTop: 8 }}>
            <summary className="row" style={{ gap: 10, cursor: 'pointer', minHeight: 44 }}>
              <b className="t14">Versão {v.numero}</b>
              <Chip cor={s.cor} texto={v.numero === vigente && v.situacao === 'assinada' ? 'Vigente' : s.texto} />
              <span className="t13 mute">
                {v.assinada_em === null || v.assinada_em === undefined ? '' : `Assinada em ${formatarDataHora(v.assinada_em, FUSO_DA_CLINICA)}`}
              </span>
            </summary>
            {v.motivo !== null && v.motivo !== undefined && <p className="t13" style={{ margin: '6px 0' }}>Motivo da retificação: {v.motivo}</p>}
            {v.hash_documento !== null && v.hash_documento !== undefined && <p className="t13 mute num" style={{ margin: '6px 0' }}>Hash do documento: {v.hash_documento}</p>}
            <pre className="texto-laudo">{v.texto}</pre>
          </details>
        );
      })}
    </div>
  );
}

function EditorDoRascunho({ laudo, versao, retomada }: { laudo: Laudo; versao: VersaoDoLaudo; retomada: Retomada | null }) {
  const r = useRascunho(laudo, versao.numero, versao.texto);
  const [assinando, setAssinando] = useState(retomada?.retomar === 'assinar');
  return (
    <>
      {versao.motivo !== null && versao.motivo !== undefined && (
        <Aviso nivel="informativo" titulo={`Retificação, versão ${String(versao.numero)}.`}>
          Motivo: {versao.motivo}. Ao assinar, a versão anterior fica marcada como substituída.
        </Aviso>
      )}
      <AreaDeTexto rotulo={`Texto do laudo, versão ${String(versao.numero)}`} value={r.texto} onChange={r.mudar} linhas={12} />
      <div className="row wrap" style={{ gap: 10 }}>
        <SituacaoDoRascunho r={r} />
        <div className="grow" />
        <Botao
          variante="principal"
          onPress={() => {
            setAssinando(true);
          }}
        >
          Assinar (ICP-Brasil)
        </Botao>
      </div>
      <PainelDeModelos
        laudo={laudo}
        aoInserir={(t) => {
          r.mudar(r.texto.trim() === '' ? t : `${r.texto}\n\n${t}`);
        }}
      />
      <PainelDeAchados
        laudo={laudo}
        aoLevarAoTexto={(t) => {
          r.mudar(r.texto.trim() === '' ? t : `${r.texto}\n\n${t}`);
        }}
      />
      {assinando && (
        <DialogoDeAssinatura
          laudo={laudo}
          retomada={retomada}
          antesDeAssinar={r.salvarAgora}
          aoFechar={() => {
            setAssinando(false);
          }}
        />
      )}
    </>
  );
}

function ConversarAntes({ laudo }: { laudo: Laudo }) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const m = useMutation({
    mutationFn: (marcado: boolean) => api.laudos.conversarAntes(laudo.id, { marcado }),
    onSuccess: (l) => {
      queryClient.setQueryData(chaves.laudo(laudo.id), l);
    },
  });
  return (
    <div className="col" style={{ gap: 6 }}>
      <CaixaDeMarcar
        rotulo="Conversar com o paciente antes de liberar"
        detalhe="Resultado delicado não vai direto ao portal: fica retido até você liberar."
        marcado={laudo.conversar_antes}
        desabilitado={m.isPending || laudo.publicado_no_portal}
        aoMudar={(v) => {
          m.mutate(v);
        }}
      />
      {m.isError && <ErroDoAto erro={m.error} ato="marcar a conversa antes da liberação" />}
    </div>
  );
}

/** Laudo impresso: só versão assinada; com retificação em rascunho, sai a vigente. */
function PdfDoLaudo({ laudo }: { laudo: Laudo }) {
  const { api } = useDependencias();
  return <BaixarDocumento rotulo="Baixar PDF" ato="baixar o PDF do laudo" baixar={() => api.laudos.baixarPdf(laudo.id)} />;
}

function ConteudoDoLaudo({ laudo, retomada }: { laudo: Laudo; retomada: Retomada | null }) {
  const [retificando, setRetificando] = useState(retomada?.retomar === 'retificar');
  const rascunho = laudo.versoes.find((v) => v.situacao === 'rascunho');
  const vigente = laudo.versoes.find((v) => v.numero === laudo.versao_vigente);
  const sit = SITUACAO_DO_LAUDO[laudo.situacao] ?? { texto: 'Situação não reconhecida', cor: 'cinza' as const };
  return (
    <div className="col" style={{ gap: 14 }}>
      <div className="row wrap" style={{ gap: 10 }}>
        <h2 className="t15 b">
          {laudo.exame}, <span className="num">{laudo.codigo}</span>
        </h2>
        <Chip cor={sit.cor} texto={sit.texto} />
      </div>
      <ConversarAntes laudo={laudo} />
      {rascunho !== undefined ? (
        <>
          {vigente?.situacao === 'assinada' && (
            <section className="col" aria-label={`Versão ${String(vigente.numero)} assinada (vigente)`} style={{ gap: 6 }}>
              <p className="t13 mute" style={{ margin: 0 }}>
                Até a retificação ser assinada, a versão {vigente.numero} continua valendo e é a que sai no PDF.
              </p>
              <PdfDoLaudo laudo={laudo} />
            </section>
          )}
          <EditorDoRascunho key={rascunho.numero} laudo={laudo} versao={rascunho} retomada={retomada} />
        </>
      ) : (
        vigente !== undefined && (
          <>
            <Aviso nivel="feito" titulo={`Laudo assinado, versão ${String(vigente.numero)}.`}>
              {vigente.assinada_em === null || vigente.assinada_em === undefined
                ? 'Assinado'
                : `Assinado em ${formatarDataHora(vigente.assinada_em, FUSO_DA_CLINICA)}`}
              . O texto está travado; para corrigir, crie uma retificação com motivo.
            </Aviso>
            {laudo.publicado_no_portal ? (
              <Aviso nivel="informativo" titulo="Publicado no portal do paciente." />
            ) : laudo.conversar_antes ? (
              <Aviso nivel="atencao" titulo="Retido: conversar antes de liberar." acoes={<BotaoDeLiberacao laudo={laudo} />}>
                O laudo não foi ao portal. Libere depois de conversar com o paciente.
              </Aviso>
            ) : (
              <Aviso nivel="atencao" titulo="Assinado e ainda não publicado no portal." acoes={<BotaoDePublicacao laudo={laudo} />}>
                A publicação automática não aconteceu. Publique agora para o paciente ver o resultado.
              </Aviso>
            )}
            <section aria-label={`Texto assinado, versão ${String(vigente.numero)}`}>
              <pre className="texto-laudo">{vigente.texto}</pre>
            </section>
            <PainelDeAchados laudo={laudo} />
            <div className="row wrap" style={{ gap: 10, alignItems: 'flex-start' }}>
              <Botao
                onPress={() => {
                  setRetificando(true);
                }}
              >
                Retificar
              </Botao>
              <div style={{ flex: 1, minWidth: 0 }}>
                <PdfDoLaudo laudo={laudo} />
              </div>
            </div>
            <CompartilharLaudo laudo={laudo} />
            {retificando && (
              <DialogoDeRetificacao
                laudo={laudo}
                textoVigente={vigente.texto}
                retomada={retomada}
                aoFechar={() => {
                  setRetificando(false);
                }}
              />
            )}
          </>
        )
      )}
      <Versoes versoes={laudo.versoes} vigente={laudo.versao_vigente ?? null} />
    </div>
  );
}

/** Laudo (Laudo.dc.html, FC-LAU e FC-ACE): rascunho com salvamento automático, assinatura e versões. */
export function TelaLaudo() {
  const { api } = useDependencias();
  const { laudoId } = rota.useParams();
  const retomada = useRetomada();
  const consulta = useQuery(consultaLaudo(api, laudoId));
  const erro = consulta.error;
  const semVinculo = erro instanceof ErroDaApi && erro.codigo === 'sem-vinculo-de-cuidado';
  const l = consulta.data;

  let corpo;
  if (semVinculo) {
    corpo = (
      <div className="col" style={{ gap: 14 }}>
        <SemPermissao titulo="Você não tem vínculo de cuidado com este paciente" detalhe={erro.problema.detail} />
        <AcessoDeEmergencia laudoId={laudoId} retomada={retomada} aoLiberar={() => void consulta.refetch()} />
      </div>
    );
  } else if (l === undefined) {
    corpo = (
      <EstadoDaConsulta
        oQue="o laudo"
        carregando={consulta.isPending}
        erro={erro}
        aoTentarDeNovo={() => void consulta.refetch()}
        semPermissao={{ titulo: 'Seu perfil não lê laudos', detalhe: 'A situação do laudo aparece no atendimento. O conteúdo é só da equipe médica.' }}
      />
    );
  } else {
    corpo = <ConteudoDoLaudo laudo={l} retomada={retomada} />;
  }

  return (
    <>
      <CabecalhoDaTela titulo="Laudar" />
      {l !== undefined && (
        <BarraDoPaciente
          nome={l.paciente.nome}
          nascimento={formatarData(l.paciente.nascimento)}
          resumo={`${l.exame}, ${l.codigo}${l.estudo === null || l.estudo === undefined ? '' : `, ${String(l.estudo.imagens)} imagens`}`}
          acoes={l.estudo === null || l.estudo === undefined ? undefined : <AbrirImagens key={l.estudo.id} estudoId={l.estudo.id} />}
        />
      )}
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: '300px minmax(0, 1fr)', gap: 16 }}>
        <FilaDeLaudos atual={laudoId} />
        <section className="card pane" aria-label="Laudo">
          <div className="scroll">{corpo}</div>
        </section>
      </div>
    </>
  );
}
