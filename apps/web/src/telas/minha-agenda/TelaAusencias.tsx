import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { AusenciaDoMedico, DecisaoDaAusencia } from '@vidacor/api-client';
import { Abas, Botao, Campo, Chip, formatarData, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { consultaSessao } from '../../consultas';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { NOME_DO_MOTIVO } from './PedidoDeBloqueio';

function periodo(p: AusenciaDoMedico): string {
  const ultimo = new Date(new Date(p.fim).getTime() - 1).toISOString().slice(0, 10);
  return `${formatarData(p.inicio.slice(0, 10))} a ${formatarData(ultimo)}`;
}

function Decisao({ p, podeDecidir }: { p: AusenciaDoMedico; podeDecidir: boolean }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [recusando, setRecusando] = useState(false);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((d: DecisaoDaAusencia, chave: string) => api.ausencias.decidir(p.id, d, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      await cliente.invalidateQueries({ queryKey: ['ausencias'] });
    },
  });
  if (!podeDecidir) {
    return null;
  }
  const erroMotivo = motivo.trim().length < 5 ? 'Diga o motivo ao médico (5 letras ou mais).' : null;
  return (
    <div className="col" style={{ gap: 6, marginTop: 6 }}>
      {recusando ? (
        <form
          className="col"
          style={{ gap: 6 }}
          aria-label={`Recusar o pedido de ${p.medico}`}
          noValidate
          onSubmit={(ev) => {
            ev.preventDefault();
            setTentou(true);
            if (erroMotivo === null) op.enviar({ aprovar: false, motivo: motivo.trim() });
          }}
        >
          <Campo rotulo="Motivo da recusa" value={motivo} onChange={setMotivo} erro={tentou ? erroMotivo : null} />
          <div className="row" style={{ gap: 8 }}>
            <Botao pequeno type="submit" ocupado={op.enviando}>
              Recusar o pedido
            </Botao>
            <Botao
              pequeno
              onPress={() => {
                setRecusando(false);
              }}
            >
              Voltar
            </Botao>
          </div>
        </form>
      ) : (
        <div className="row" style={{ gap: 8 }}>
          <Botao
            pequeno
            variante="principal"
            ocupado={op.enviando}
            onPress={() => {
              op.enviar({ aprovar: true });
            }}
          >
            Aprovar o bloqueio
          </Botao>
          <Botao
            pequeno
            onPress={() => {
              setRecusando(true);
            }}
          >
            Recusar
          </Botao>
        </div>
      )}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="decidir o pedido de bloqueio" />}
    </div>
  );
}

function Lista({ pedidos, podeDecidir, vazio }: { pedidos: AusenciaDoMedico[]; podeDecidir: boolean; vazio: string }) {
  if (pedidos.length === 0) {
    return <Vazio titulo={vazio} />;
  }
  return (
    <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }}>
      {pedidos.map((p) => (
        <li key={p.id} style={{ padding: '10px 0', borderBottom: '1px solid var(--line)' }}>
          <div className="row wrap" style={{ gap: 8 }}>
            <b>{p.medico}</b>
            <span className="t14">{periodo(p)}</span>
            <Chip
              cor={p.situacao === 'aguardando' ? 'ambar' : p.situacao === 'aprovada' ? 'verde' : 'cinza'}
              texto={p.situacao === 'aguardando' ? 'Aguardando' : p.situacao === 'aprovada' ? 'Aprovado' : p.situacao === 'recusada' ? 'Recusado' : 'Cancelado'}
            />
          </div>
          <span className="t13 mute" style={{ display: 'block' }}>
            {NOME_DO_MOTIVO[p.motivo]}
            {typeof p.detalhe === 'string' ? `: ${p.detalhe}` : ''}. {p.codigo}, pedido em {formatarDataHora(p.criado_em, FUSO_DA_CLINICA)}.{' '}
            {String(p.afetados_na_decisao ?? p.afetados_no_pedido)} paciente(s) agendado(s) no período.
            {typeof p.decidido_por === 'string' ? ` Decidido por ${p.decidido_por}.` : ''}
            {typeof p.motivo_decisao === 'string' ? ` Motivo: ${p.motivo_decisao}` : ''}
          </span>
          {p.situacao === 'aguardando' && <Decisao p={p} podeDecidir={podeDecidir} />}
        </li>
      ))}
    </ul>
  );
}

/**
 * Ausências dos médicos: a coordenação (gerência) aprova ou recusa os pedidos de bloqueio da agenda; a
 * recepção acompanha para remarcar os pacientes afetados. O servidor confere quem pode decidir.
 */
export function TelaAusencias() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const q = useQuery({ queryKey: ['ausencias'], queryFn: () => api.ausencias.listar() });
  const podeDecidir = sessao.data?.usuario.perfis.includes('gerente') ?? false;
  if (q.data === undefined) {
    return (
      <>
        <CabecalhoDaTela titulo="Ausências dos médicos" />
        <EstadoDaConsulta oQue="os pedidos de bloqueio" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} />
      </>
    );
  }
  const aguardando = q.data.filter((p) => p.situacao === 'aguardando');
  const aprovadas = q.data.filter((p) => p.situacao === 'aprovada');
  const outras = q.data.filter((p) => p.situacao === 'recusada' || p.situacao === 'cancelada');
  return (
    <>
      <CabecalhoDaTela
        titulo="Ausências dos médicos"
        complemento={<span className="t14 mute">O bloqueio só fecha a agenda depois da aprovação. Quem pede nunca aprova.</span>}
      />
      <section className="card pane fill" aria-label="Pedidos de bloqueio" style={{ minHeight: 0 }}>
        <Abas
          rotulo="Pedidos de bloqueio"
          abas={[
            { id: 'aguardando', rotulo: `Aguardando ${String(aguardando.length)}`, conteudo: <Lista pedidos={aguardando} podeDecidir={podeDecidir} vazio="Nenhum pedido aguardando." /> },
            { id: 'aprovadas', rotulo: `Aprovadas ${String(aprovadas.length)}`, conteudo: <Lista pedidos={aprovadas} podeDecidir={false} vazio="Nenhuma ausência aprovada." /> },
            { id: 'outras', rotulo: 'Recusadas e canceladas', conteudo: <Lista pedidos={outras} podeDecidir={false} vazio="Nada por aqui." /> },
          ]}
        />
      </section>
    </>
  );
}
