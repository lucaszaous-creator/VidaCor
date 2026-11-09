import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { CasoDeRevisao as Caso, ClasseRadpeer, ParecerDeRevisao } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, Campo, Dialogo, formatarData, formatarDataHora, GrupoDeOpcoes, ListaDeDados } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { AbrirImagens } from '../laudos/AbrirImagens';

export const CHAVE_FILA_REVISAO = ['revisao-por-pares', 'fila'] as const;

/** Escala RADPEER (Revisao-Pares.dc.html). */
export const RADPEER: { classe: ClasseRadpeer; texto: string }[] = [
  { classe: '1', texto: 'Concorda com a interpretação' },
  { classe: '2a', texto: 'Discrepância difícil de perceber, sem impacto clínico' },
  { classe: '2b', texto: 'Discrepância difícil de perceber, pode ter impacto clínico' },
  { classe: '3a', texto: 'Discrepância que deveria ser percebida, sem impacto clínico' },
  { classe: '3b', texto: 'Discrepância que deveria ser percebida, pode ter impacto clínico' },
];
export const SIGNIFICATIVA = new Set<string>(['2b', '3a', '3b']);

function Conflito({ caso }: { caso: Caso }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const declarar = useMutation({
    mutationFn: () => api.revisaoPorPares.conflito(caso.id, motivo.trim()),
    onSuccess: () => cliente.invalidateQueries({ queryKey: ['revisao-por-pares'] }),
  });
  const ok = motivo.trim().length >= 5;
  if (declarar.isSuccess) {
    return <Aviso nivel="feito" titulo="Conflito declarado.">O caso volta para a coordenação médica designar outro revisor.</Aviso>;
  }
  return (
    <details>
      <summary className="t14" style={{ cursor: 'pointer', minHeight: 44, display: 'flex', alignItems: 'center' }}>
        Declarar conflito e passar a outro revisor
      </summary>
      <div className="col" style={{ gap: 8 }}>
        <Campo rotulo="Qual é o conflito" value={motivo} onChange={setMotivo} ajuda="Ex.: atendeu o paciente, parentesco." erro={tentou && !ok ? 'Escreva o motivo (5 letras ou mais).' : null} />
        {declarar.isError && <ErroDoAto erro={declarar.error} ato="declarar o conflito" />}
        <div className="row">
          <Botao
            ocupado={declarar.isPending}
            onPress={() => {
              setTentou(true);
              if (ok) declarar.mutate();
            }}
          >
            Declarar conflito
          </Botao>
        </div>
      </div>
    </details>
  );
}

function Parecer({ caso }: { caso: Caso }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [classe, setClasse] = useState<ClasseRadpeer | null>(null);
  const [comentario, setComentario] = useState('');
  const [tentou, setTentou] = useState(false);
  const [confirmando, setConfirmando] = useState(false);
  const op = useOperacao((p: ParecerDeRevisao, chave: string) => api.revisaoPorPares.parecer(caso.id, p, { idempotencyKey: chave }), {
    aoConcluir: async (c) => {
      setConfirmando(false);
      cliente.setQueryData(['revisao-por-pares', 'caso', caso.id], c);
      await cliente.invalidateQueries({ queryKey: CHAVE_FILA_REVISAO });
    },
    aoFalhar: () => {
      setConfirmando(false);
    },
  });
  const comentarioOk = classe === '1' || comentario.trim().length >= 10;
  const escolhida = RADPEER.find((r) => r.classe === classe);
  return (
    <div className="col" style={{ gap: 10 }}>
      <GrupoDeOpcoes
        rotulo="Concordância (RADPEER)"
        valor={classe}
        aoMudar={(v) => {
          setClasse(RADPEER.find((r) => r.classe === v)?.classe ?? null);
        }}
        opcoes={RADPEER.map((r) => ({ valor: r.classe, rotulo: r.classe }))}
      />
      <dl className="t13" style={{ margin: 0, display: 'grid', gridTemplateColumns: '32px minmax(0, 1fr)', gap: '2px 8px' }}>
        {RADPEER.map((r) => (
          <div key={r.classe} style={{ display: 'contents' }}>
            <dt className={r.classe === classe ? 'b' : 'mute'}>{r.classe}</dt>
            <dd className={r.classe === classe ? 'b' : 'mute'} style={{ margin: 0 }}>
              {r.texto}
            </dd>
          </div>
        ))}
      </dl>
      <AreaDeTexto
        rotulo="Comentário educativo para o colega"
        value={comentario}
        onChange={setComentario}
        linhas={3}
        erro={tentou && !comentarioOk ? 'Na discrepância, explique o que foi visto (10 letras ou mais).' : null}
      />
      {classe !== null && SIGNIFICATIVA.has(classe) && (
        <Aviso nivel="atencao" titulo={`Discrepância significativa (${classe}).`}>
          Ao enviar, o médico que laudou recebe a pendência. O laudo original não muda; se houver impacto, quem laudou faz a retificação com motivo.
        </Aviso>
      )}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="enviar a revisão" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao
          variante="principal"
          isDisabled={classe === null}
          onPress={() => {
            setTentou(true);
            if (classe !== null && comentarioOk) setConfirmando(true);
          }}
        >
          Enviar revisão
        </Botao>
        {classe === null && <span className="t13 mute">Escolha uma classificação para enviar.</span>}
      </div>
      <Dialogo
        aberto={confirmando}
        titulo={`Enviar a revisão do laudo de ${caso.paciente.nome} com classificação ${classe ?? ''}?`}
        paciente={{ nome: caso.paciente.nome, nascimento: formatarData(caso.paciente.nascimento) }}
        rotuloConfirmar="Enviar revisão"
        ocupado={op.enviando}
        aoConfirmar={() => {
          if (classe !== null) op.enviar({ classe, comentario: comentario.trim() });
        }}
        aoFechar={() => {
          setConfirmando(false);
        }}
      >
        <p style={{ margin: 0 }}>
          {caso.exame}, {caso.laudo_codigo}, revisão {caso.codigo}. {escolhida?.texto}. Depois de enviada, a revisão não muda.
        </p>
      </Dialogo>
    </div>
  );
}

/** Caso aberto para o revisor: dois identificadores, laudo assinado (só leitura) e o parecer. */
export function CasoDeRevisao({ id }: { id: string }) {
  const { api } = useDependencias();
  const consulta = useQuery({ queryKey: ['revisao-por-pares', 'caso', id], queryFn: () => api.revisaoPorPares.obter(id) });
  const c = consulta.data;
  if (c === undefined) {
    return <EstadoDaConsulta oQue="o caso" carregando={consulta.isPending} erro={consulta.error} aoTentarDeNovo={() => void consulta.refetch()} />;
  }
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row wrap" style={{ gap: 10 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {c.exame}
        </h2>
        <span className="t13 mute num">
          {c.codigo}, {c.laudo_codigo}
        </span>
        <div className="grow" />
        {c.estudo_id !== null && c.estudo_id !== undefined && <AbrirImagens estudoId={c.estudo_id} />}
      </div>
      <ListaDeDados
        larguraDoRotulo={130}
        itens={[
          { rotulo: 'Paciente', valor: `${c.paciente.nome}, nascido(a) em ${formatarData(c.paciente.nascimento)}` },
          { rotulo: 'Laudado por', valor: c.autor ?? 'Oculto até o envio' },
          { rotulo: 'Versão', valor: `${String(c.versao)}${c.assinada_em === null || c.assinada_em === undefined ? '' : `, assinada em ${formatarDataHora(c.assinada_em, FUSO_DA_CLINICA)}`}` },
        ]}
      />
      {c.texto !== null && c.texto !== undefined && (
        <section aria-label="Laudo original (versão assinada, somente leitura)" className="col" style={{ gap: 4 }}>
          <h3 className="t14 b" style={{ margin: 0 }}>
            Laudo original <span className="mute">(versão assinada, somente leitura)</span>
          </h3>
          <pre className="texto-laudo">{c.texto}</pre>
        </section>
      )}
      {c.situacao === 'designada' && (
        <>
          <Parecer caso={c} />
          <Conflito caso={c} />
        </>
      )}
      {c.situacao === 'concluida' && (
        <Aviso nivel="feito" titulo={`Revisão ${c.codigo} enviada com classificação ${c.classe ?? ''}.`}>
          Registrada no lastro. Autor revelado: {c.autor ?? 'não informado'}.{c.comentario === null || c.comentario === undefined || c.comentario === '' ? '' : ` Seu comentário: ${c.comentario}`}
        </Aviso>
      )}
    </div>
  );
}
