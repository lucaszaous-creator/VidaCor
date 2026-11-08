import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import type { ContratoDeConvenio } from '@vidacor/api-client';
import { Abas, Aviso, Botao, Chip, formatarData, formatarDinheiro, Vazio } from '@vidacor/design-system';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { chavesDoFaturamento, DetalheDaConta } from './DetalheDaConta';
import { DetalheDaGlosa, DetalheDoLote } from './DetalheDoLote';
import { SITUACAO_DA_CONTA, SITUACAO_DA_GLOSA, SITUACAO_DO_LOTE } from './rotulos';

const rota = getRouteApi('/app/faturamento');
export type AbaDoFaturamento = 'guias' | 'lotes' | 'glosas';

/**
 * Faturamento de convênios (Faturamento.dc.html): guias com situação, prazo e pendências; lotes TISS
 * com envio e arquivo; glosas com recurso. A conferência, o lote e os valores são do servidor (G1);
 * esconder botão para quem não é faturista é só conforto: cada rota confere o ato.
 */
export function TelaFaturamento() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const navegar = useNavigate();
  const sessao = useQuery(consultaSessao(api));
  const faturista = sessao.data?.usuario.perfis.includes('faturista') ?? false;
  const aba: AbaDoFaturamento = busca.aba ?? 'guias';
  const abrir = (proxima: { aba: AbaDoFaturamento; id?: string }) => {
    void navegar({ to: '/faturamento', search: proxima, replace: true });
  };
  const contas = useQuery({ queryKey: chavesDoFaturamento.contas, queryFn: () => api.faturamento.contas() });
  const lotes = useQuery({ queryKey: chavesDoFaturamento.lotes, queryFn: () => api.faturamento.lotes() });
  const glosas = useQuery({ queryKey: chavesDoFaturamento.glosas, queryFn: () => api.faturamento.glosas() });
  const glosa = glosas.data?.find((g) => g.id === busca.id) ?? null;
  const semPermissao = { titulo: 'Você não vê o faturamento', detalhe: 'O faturamento de convênios é do faturista e da gerência.' };

  const listaDeGuias = (
    <div>
      <EstadoDaConsulta oQue="as guias" carregando={contas.isPending} erro={contas.error} aoTentarDeNovo={() => void contas.refetch()} semPermissao={semPermissao} />
      {contas.data?.length === 0 && <Vazio titulo="Nenhuma guia para faturar." detalhe="Confira as guias para trazer os atendimentos com guia consumida." />}
      {contas.data?.map((c) => {
        const s = SITUACAO_DA_CONTA[c.situacao];
        const pend = c.pendencias_que_impedem > 0 ? `${String(c.pendencias_que_impedem)} pendência(s)` : null;
        return (
          <LinhaDoPaciente
            key={c.id}
            nome={c.paciente.nome}
            nascimento={c.paciente.nascimento}
            detalhe={`${c.operadora}, guia ${c.numero_guia_operadora}, ${formatarDinheiro(c.valor_total_centavos)}${c.enviar_ate == null ? '' : `, enviar até ${formatarData(c.enviar_ate)}`}`}
            situacao={s}
            {...(pend === null ? {} : { marca: pend })}
            selecionada={aba === 'guias' && c.id === busca.id}
            aoAbrir={() => {
              abrir({ aba: 'guias', id: c.id });
            }}
            contextoDoMenu={c.codigo}
          />
        );
      })}
    </div>
  );

  const listaDeLotes = (
    <div className="col" style={{ gap: 8 }}>
      {faturista && <GerarLote />}
      <EstadoDaConsulta oQue="os lotes" carregando={lotes.isPending} erro={lotes.error} aoTentarDeNovo={() => void lotes.refetch()} semPermissao={semPermissao} />
      {lotes.data?.length === 0 && <Vazio titulo="Nenhum lote gerado." />}
      {lotes.data?.map((l) => (
        <button
          key={l.id}
          type="button"
          className="lp"
          aria-current={aba === 'lotes' && l.id === busca.id}
          onClick={() => {
            abrir({ aba: 'lotes', id: l.id });
          }}
        >
          <span style={{ flex: 1, minWidth: 0 }}>
            <span className="t" style={{ display: 'block' }}>
              Lote {l.numero}, {l.operadora}
            </span>
            <span className="s" style={{ display: 'block' }}>
              {l.quantidade_guias} guia(s), {formatarDinheiro(l.valor_total_centavos)}
              {l.envio_pendente != null && ', envio pendente'}
            </span>
          </span>
          <Chip cor={SITUACAO_DO_LOTE[l.situacao].cor} texto={SITUACAO_DO_LOTE[l.situacao].texto} />
        </button>
      ))}
    </div>
  );

  const listaDeGlosas = (
    <div>
      <EstadoDaConsulta oQue="as glosas" carregando={glosas.isPending} erro={glosas.error} aoTentarDeNovo={() => void glosas.refetch()} semPermissao={semPermissao} />
      {glosas.data?.length === 0 && <Vazio titulo="Nenhuma glosa recebida." />}
      {glosas.data?.map((g) => (
        <LinhaDoPaciente
          key={g.id}
          nome={g.paciente.nome}
          nascimento={g.paciente.nascimento}
          detalhe={`glosa ${g.codigo_glosa}, ${formatarDinheiro(g.valor_centavos)}, recurso até ${formatarData(g.prazo_recurso)}`}
          situacao={SITUACAO_DA_GLOSA[g.situacao]}
          selecionada={aba === 'glosas' && g.id === busca.id}
          aoAbrir={() => {
            abrir({ aba: 'glosas', id: g.id });
          }}
          contextoDoMenu={g.conta_codigo}
        />
      ))}
    </div>
  );

  let detalhe = <Vazio titulo="Escolha uma guia, um lote ou uma glosa." detalhe="A situação e as pendências vêm da conferência do servidor." />;
  if (busca.id !== undefined && aba === 'guias') detalhe = <DetalheDaConta key={busca.id} id={busca.id} podeCorrigir={faturista} />;
  if (busca.id !== undefined && aba === 'lotes') detalhe = <DetalheDoLote key={busca.id} id={busca.id} podeEnviar={faturista} />;
  if (glosa !== null && aba === 'glosas') detalhe = <DetalheDaGlosa key={glosa.id} glosa={glosa} podeRecorrer={faturista} />;

  return (
    <>
      <CabecalhoDaTela titulo="Faturamento de convênios" complemento={<span className="t14 mute">TISS 4.03.00, conferido no servidor antes de cada lote</span>}>
        {faturista && <Conferir />}
      </CabecalhoDaTela>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Guias, lotes e glosas">
          <div className="scroll">
            <Abas
              rotulo="Faturamento"
              selecionada={aba}
              aoSelecionar={(id) => {
                abrir({ aba: id as AbaDoFaturamento });
              }}
              abas={[
                { id: 'guias', rotulo: 'Guias', conteudo: listaDeGuias },
                { id: 'lotes', rotulo: 'Lotes', conteudo: listaDeLotes },
                { id: 'glosas', rotulo: 'Glosas', conteudo: listaDeGlosas },
              ]}
            />
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe">
          <div className="scroll col" style={{ gap: 14 }}>
            {detalhe}
          </div>
        </section>
      </div>
    </>
  );
}

/** Conferir guias: o servidor apura as novas e confere todas pelas regras de cada operadora. */
export function Conferir() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const op = useOperacao((_: undefined, chave: string) => api.faturamento.conferir({ idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: chavesDoFaturamento.tudo }),
  });
  return (
    <div className="row wrap" style={{ gap: 10 }}>
      {op.resultado !== null && (
        <span className="t14" role="status">
          {`${String(op.resultado.conferidas)} guia(s) conferida(s): ${String(op.resultado.prontas)} pronta(s), ${String(op.resultado.com_pendencia)} com pendência.`}
        </span>
      )}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="conferir as guias" />}
      <Botao
        variante="principal"
        ocupado={op.enviando}
        onPress={() => {
          op.reiniciar();
          op.enviar(undefined);
        }}
      >
        Conferir guias
      </Botao>
    </div>
  );
}

/** Gerar lote por operadora com as guias prontas (o servidor confere cada uma de novo). */
function GerarLote() {
  const { api } = useDependencias();
  const contratos = useQuery({ queryKey: chavesDoFaturamento.contratos, queryFn: () => api.faturamento.contratos() });
  if (contratos.data === undefined) {
    return <EstadoDaConsulta oQue="os contratos" carregando={contratos.isPending} erro={contratos.error} aoTentarDeNovo={() => void contratos.refetch()} linhas={1} />;
  }
  if (contratos.data.length === 0) {
    return (
      <Aviso nivel="atencao" titulo="Nenhum contrato de convênio cadastrado.">
        Sem contrato, as guias ficam bloqueadas na auditoria. O cadastro de contratos é feito pela gerência.
      </Aviso>
    );
  }
  return (
    <div className="col" style={{ gap: 6, padding: '4px 0 8px' }}>
      {contratos.data.map((k) => (
        <GerarLoteDe key={k.id} contrato={k} />
      ))}
    </div>
  );
}

function GerarLoteDe({ contrato }: { contrato: ContratoDeConvenio }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const navegar = useNavigate();
  const op = useOperacao((_: undefined, chave: string) => api.faturamento.gerarLote({ contrato_id: contrato.id }, { idempotencyKey: chave }), {
    aoConcluir: async (d) => {
      await cliente.invalidateQueries({ queryKey: chavesDoFaturamento.tudo });
      void navegar({ to: '/faturamento', search: { aba: 'lotes', id: d.lote.id }, replace: true });
    },
  });
  return (
    <div className="col" style={{ gap: 4 }}>
      <div className="row wrap" style={{ gap: 10 }}>
        <span className="t14" style={{ flex: 1, minWidth: 0 }}>
          {contrato.operadora}: {contrato.contas_prontas} guia(s) pronta(s), regras {contrato.regra_versao}
        </span>
        <Botao
          pequeno
          ocupado={op.enviando}
          isDisabled={contrato.contas_prontas === 0}
          onPress={() => {
            op.reiniciar();
            op.enviar(undefined);
          }}
        >
          {`Gerar lote ${contrato.operadora}`}
        </Botao>
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="gerar o lote" />}
    </div>
  );
}
