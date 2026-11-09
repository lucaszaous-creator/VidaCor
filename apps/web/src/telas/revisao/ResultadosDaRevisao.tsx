import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { PainelDeRevisao, RevisaoDesignada, RevisaoDoMeuLaudo } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Chip, Escolha, formatarData, Tabela, Vazio, type CorDoChip } from '@vidacor/design-system';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { RADPEER, SIGNIFICATIVA } from './CasoDeRevisao';

const PENDENCIA: Record<string, { texto: string; cor: CorDoChip }> = {
  aguarda_autor: { texto: 'Aguarda o autor', cor: 'ambar' },
  retificado: { texto: 'Retificado', cor: 'azul' },
  mantido: { texto: 'Laudo mantido', cor: 'cinza' },
};

const pendencia = (p: string | null | undefined): { texto: string; cor: CorDoChip } =>
  p === null || p === undefined ? { texto: 'Educativo', cor: 'cinza' } : (PENDENCIA[p] ?? { texto: 'Situação não reconhecida', cor: 'cinza' });

function DecisaoDoAutor({ r }: { r: RevisaoDoMeuLaudo }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const navegar = useNavigate();
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const decidir = useMutation({
    mutationFn: (decisao: 'retificado' | 'mantido') => api.revisaoPorPares.decidirPendencia(r.id, { decisao, motivo: motivo.trim() === '' ? null : motivo.trim() }),
    onSuccess: () => cliente.invalidateQueries({ queryKey: ['revisao-por-pares', 'meus'] }),
  });
  const ok = motivo.trim().length >= 5;
  return (
    <div className="col" style={{ gap: 8 }}>
      <Campo rotulo={`Motivo para ${r.laudo_codigo}`} value={motivo} onChange={setMotivo} ajuda="Obrigatório para manter o laudo como está." erro={tentou && !ok ? 'Para manter, explique por quê (5 letras ou mais).' : null} />
      {decidir.isError && <ErroDoAto erro={decidir.error} ato="decidir a pendência" />}
      <div className="row wrap" style={{ gap: 8 }}>
        <Botao pequeno onPress={() => void navegar({ to: '/laudos/$laudoId', params: { laudoId: r.laudo_id } })}>
          Abrir o laudo para retificar
        </Botao>
        <Botao pequeno variante="principal" ocupado={decidir.isPending} onPress={() => { decidir.mutate('retificado'); }}>
          Já retifiquei
        </Botao>
        <Botao
          pequeno
          onPress={() => {
            setTentou(true);
            if (ok) decidir.mutate('mantido');
          }}
        >
          Manter o laudo
        </Botao>
      </div>
    </div>
  );
}

/** Revisões dos laudos do médico: o revisor não é revelado; discrepância significativa pede decisão. */
export function MeusLaudosRevisados() {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['revisao-por-pares', 'meus'], queryFn: () => api.revisaoPorPares.meusLaudos() });
  const d = q.data;
  if (d === undefined) return <EstadoDaConsulta oQue="as revisões dos seus laudos" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} />;
  return (
    <div className="col" style={{ gap: 10 }}>
      <p className="t14" style={{ margin: 0 }}>
        {d.revisados} laudo(s) seu(s) revisado(s)
        {d.concordancia_percentual === null || d.concordancia_percentual === undefined ? '' : `, ${String(d.concordancia_percentual)}% de concordância`}, {d.pendentes} pendência(s) para você decidir.
      </p>
      {d.revisoes.length === 0 && <Vazio titulo="Nenhum laudo seu revisado ainda." />}
      <ul style={{ listStyle: 'none', margin: 0, padding: 0 }} aria-label="Revisões dos meus laudos">
        {d.revisoes.map((r) => {
          const p = pendencia(r.pendencia);
          return (
            <li key={r.id} className="col" style={{ gap: 6, padding: '10px 0', borderBottom: '1px solid var(--line)' }}>
              <div className="row wrap" style={{ gap: 8 }}>
                <b>
                  {r.exame}, <span className="num">{r.laudo_codigo}</span>
                </b>
                <Chip cor={r.classe === '1' ? 'verde' : SIGNIFICATIVA.has(r.classe) ? 'rosa' : 'ambar'} texto={`Classe ${r.classe}`} />
                {r.classe !== '1' && <Chip cor={p.cor} texto={p.texto} />}
              </div>
              <span className="t13 mute">
                {r.paciente.nome}, {formatarData(r.paciente.nascimento)}. {RADPEER.find((x) => x.classe === r.classe)?.texto}.
              </span>
              {r.comentario !== null && r.comentario !== undefined && r.comentario !== '' && <p className="t14" style={{ margin: 0 }}>{r.comentario}</p>}
              {r.pendencia_motivo !== null && r.pendencia_motivo !== undefined && <span className="t13 mute">Sua decisão: {r.pendencia_motivo}</span>}
              {r.pendencia === 'aguarda_autor' && <DecisaoDoAutor r={r} />}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function Sorteio() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [quantidade, setQuantidade] = useState('6');
  const [dias, setDias] = useState('30');
  const q = Number(quantidade);
  const d = Number(dias);
  const ok = Number.isInteger(q) && q >= 1 && q <= 50 && Number.isInteger(d) && d >= 1 && d <= 90;
  const op = useOperacao((p: { quantidade: number; dias: number }, chave: string) => api.revisaoPorPares.sortear(p, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['revisao-por-pares'] }),
  });
  return (
    <form
      className="col"
      style={{ gap: 8 }}
      aria-label="Sortear laudos para revisão"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        if (ok) op.enviar({ quantidade: q, dias: d });
      }}
    >
      <div className="row wrap" style={{ gap: 10, alignItems: 'flex-end' }}>
        <div style={{ width: 150 }}>
          <Campo rotulo="Quantos laudos" value={quantidade} onChange={setQuantidade} />
        </div>
        <div style={{ width: 150 }}>
          <Campo rotulo="Assinados nos últimos dias" value={dias} onChange={setDias} />
        </div>
        <Botao type="submit" variante="principal" ocupado={op.enviando} isDisabled={!ok}>
          Sortear
        </Botao>
      </div>
      {!ok && <span className="t13 mute">Use de 1 a 50 laudos e de 1 a 90 dias.</span>}
      <div aria-live="polite">
        {op.resultado !== null && (
          <Aviso nivel="feito" titulo={`${String(op.resultado.designadas.length)} laudo(s) designado(s) para revisão.`}>
            {op.resultado.candidatos} candidato(s) no período. O autor nunca revisa o próprio laudo.
            {op.resultado.sem_revisor > 0 ? ` ${String(op.resultado.sem_revisor)} ficaram sem outro médico disponível.` : ''}
          </Aviso>
        )}
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="sortear os laudos" />}
      </div>
    </form>
  );
}

function Redesignar({ r, medicos }: { r: RevisaoDesignada; medicos: PainelDeRevisao['medicos'] }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [revisor, setRevisor] = useState('');
  const op = useOperacao((p: { laudo_id: string; revisor_id: string }, chave: string) => api.revisaoPorPares.designar(p, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['revisao-por-pares'] }),
  });
  return (
    <div className="col" style={{ gap: 6 }}>
      <div className="row wrap" style={{ gap: 8, alignItems: 'flex-end' }}>
        <div style={{ width: 240 }}>
          <Escolha
            rotulo={`Outro revisor para ${r.laudo_codigo}`}
            valor={revisor}
            aoMudar={setRevisor}
            vazio="Escolha o médico"
            opcoes={medicos.filter((m) => m.id !== r.revisor_id).map((m) => ({ valor: m.id, rotulo: m.nome }))}
          />
        </div>
        <Botao pequeno ocupado={op.enviando} isDisabled={revisor === ''} onPress={() => { op.enviar({ laudo_id: r.laudo_id, revisor_id: revisor }); }}>
          Designar
        </Botao>
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="designar o revisor" />}
    </div>
  );
}

/** Visão da coordenação médica: sorteio, resultado por médico e discrepâncias (sem texto de laudo). */
export function ResultadosDaRevisao() {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['revisao-por-pares', 'painel'], queryFn: () => api.revisaoPorPares.painel() });
  const p = q.data;
  if (p === undefined) {
    return (
      <EstadoDaConsulta
        oQue="os resultados"
        carregando={q.isPending}
        erro={q.error}
        aoTentarDeNovo={() => void q.refetch()}
        semPermissao={{ titulo: 'Só a coordenação médica vê os resultados', detalhe: 'O resultado individual é confidencial: só o próprio médico e a coordenação médica o veem.' }}
      />
    );
  }
  return (
    <div className="col" style={{ gap: 14 }}>
      <Sorteio />
      <Tabela
        rotulo={`Resultado por médico, últimos ${String(p.dias)} dias`}
        vazio="Nenhuma revisão concluída no período."
        colunas={[
          { id: 'medico', rotulo: 'Médico', cabecalhoDaLinha: true, celula: (r) => r.medico },
          { id: 'n', rotulo: 'Revisados', direita: true, celula: (r) => <span className="num">{r.revisados}</span> },
          { id: 'pct', rotulo: 'Concordância', direita: true, celula: (r) => <span className="num">{r.concordancia_percentual === null || r.concordancia_percentual === undefined ? 'sem dado' : `${String(r.concordancia_percentual)}%`}</span> },
          { id: 'disc', rotulo: 'Discrepâncias', celula: (r) => (r.discrepancias === 0 ? 'Nenhuma' : `${String(r.discrepancias)}, ${String(r.pendentes)} aguardando o autor`) },
        ]}
        linhas={p.por_medico.map((r) => ({ ...r, id: r.medico_id }))}
      />
      <p className="t13 mute" style={{ margin: 0 }}>
        O resultado individual não entra em avaliação de desempenho nem em repasse. Cada abertura fica no lastro.
      </p>
      <Tabela
        rotulo="Discrepâncias"
        vazio="Nenhuma discrepância no período."
        colunas={[
          { id: 'classe', rotulo: 'Classe', semQuebra: true, celula: (r) => <b className="num">{r.classe}</b> },
          { id: 'caso', rotulo: 'Caso', celula: (r) => `${r.exame}, ${r.laudo_codigo}, ${r.autor}` },
          { id: 'sit', rotulo: 'Situação', semQuebra: true, celula: (r) => <Chip cor={pendencia(r.pendencia).cor} texto={pendencia(r.pendencia).texto} /> },
        ]}
        linhas={p.discrepancias}
      />
      <Tabela
        rotulo="Revisões em aberto"
        vazio="Nenhuma revisão em aberto."
        colunas={[
          { id: 'caso', rotulo: 'Caso', celula: (r) => `${r.exame}, ${r.laudo_codigo}` },
          { id: 'rev', rotulo: 'Revisor', celula: (r) => r.revisor },
          { id: 'modo', rotulo: 'Como', semQuebra: true, celula: (r) => (r.modo === 'sorteio' ? 'Sorteio' : 'Escolha') },
        ]}
        linhas={p.abertas}
      />
      {p.conflitos.length > 0 && (
        <section className="col" aria-label="Conflitos declarados" style={{ gap: 8 }}>
          <h3 className="t15 b" style={{ margin: 0 }}>
            Conflitos declarados
          </h3>
          {p.conflitos.map((r) => (
            <div key={r.id} className="col" style={{ gap: 4 }}>
              <span className="t14">
                {r.exame}, {r.laudo_codigo}: {r.revisor} declarou conflito ({r.motivo_conflito ?? 'sem motivo'}).
              </span>
              <Redesignar r={r} medicos={p.medicos} />
            </div>
          ))}
        </section>
      )}
    </div>
  );
}
