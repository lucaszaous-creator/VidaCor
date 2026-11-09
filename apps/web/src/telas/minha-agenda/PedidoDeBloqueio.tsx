import { parseDate, type CalendarDate } from '@internationalized/date';
import { useQueryClient } from '@tanstack/react-query';
import type { AusenciaDoMedico, MotivoDaAusencia, NovaAusencia } from '@vidacor/api-client';
import { Aviso, AreaDeTexto, Botao, CampoDeData, Escolha, Gaveta } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

export const NOME_DO_MOTIVO: Record<MotivoDaAusencia, string> = {
  congresso: 'Congresso ou curso',
  ferias: 'Férias',
  licenca: 'Licença médica',
  pessoal: 'Compromisso pessoal',
};

const MOTIVOS = (Object.keys(NOME_DO_MOTIVO) as MotivoDaAusencia[]).map((m) => ({ valor: m, rotulo: NOME_DO_MOTIVO[m] }));

/** Início do dia em Brasília (sem horário de verão desde 2019), no formato RFC 3339. */
export function inicioDoDia(d: CalendarDate): string {
  return `${d.toString()}T00:00:00-03:00`;
}

/**
 * Pedido de bloqueio da própria agenda (Agenda-Medico.dc.html, "Bloquear horário"): dias inteiros, com
 * motivo. Só vale depois da aprovação da coordenação; até lá a agenda continua aberta.
 */
export function PedidoDeBloqueio({ hoje, aoFechar }: { hoje: string; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [de, setDe] = useState<CalendarDate | null>(null);
  const [ate, setAte] = useState<CalendarDate | null>(null);
  const [motivo, setMotivo] = useState('');
  const [detalhe, setDetalhe] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovaAusencia, chave: string) => api.minhaAgenda.pedirAusencia(n, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      await cliente.invalidateQueries({ queryKey: ['ausencias'] });
    },
  });
  const minimo = parseDate(hoje);
  const erroDe = de === null ? 'Escolha o primeiro dia.' : de.compare(minimo) < 0 ? 'O primeiro dia não pode ser antes de hoje.' : null;
  const erroAte =
    ate === null
      ? 'Escolha o último dia.'
      : de !== null && ate.compare(de) < 0
        ? 'O último dia vem depois do primeiro.'
        : de !== null && ate.compare(de.add({ days: 61 })) > 0
          ? 'Até 62 dias por pedido.'
          : null;
  const erroMotivo = motivo === '' ? 'Escolha o motivo para enviar.' : null;
  const feito: AusenciaDoMedico | null = op.resultado;
  return (
    <Gaveta
      titulo="Bloquear horário"
      subTitulo="Pedido para a coordenação"
      rotulo="Pedido de bloqueio da agenda"
      aoFechar={aoFechar}
    >
      {feito !== null ? (
        <div className="col" style={{ gap: 10 }}>
          <Aviso nivel="feito" titulo={`Pedido ${feito.codigo} enviado à coordenação.`}>
            {feito.afetados_no_pedido === 0
              ? 'Não há pacientes agendados com você nesse período.'
              : `${String(feito.afetados_no_pedido)} paciente(s) já agendado(s) nesse período. Com a aprovação, a recepção remarca com cada um.`}{' '}
            Enquanto não for aprovado, a agenda continua aberta.
          </Aviso>
          <div className="row">
            <Botao onPress={aoFechar}>Fechar</Botao>
          </div>
        </div>
      ) : (
        <form
          className="col"
          style={{ gap: 10 }}
          aria-label="Pedido de bloqueio"
          noValidate
          onSubmit={(ev) => {
            ev.preventDefault();
            setTentou(true);
            if (de !== null && ate !== null && erroDe === null && erroAte === null && erroMotivo === null) {
              op.enviar({
                inicio: inicioDoDia(de),
                fim: inicioDoDia(ate.add({ days: 1 })),
                motivo: motivo as MotivoDaAusencia,
                detalhe: detalhe.trim() === '' ? null : detalhe.trim(),
              });
            }
          }}
        >
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 10 }}>
            <CampoDeData rotulo="Primeiro dia" valor={de} aoMudar={setDe} erro={tentou ? erroDe : null} />
            <CampoDeData rotulo="Último dia" valor={ate} aoMudar={setAte} erro={tentou ? erroAte : null} />
          </div>
          <Escolha rotulo="Motivo" valor={motivo} aoMudar={setMotivo} vazio="Escolha o motivo" opcoes={MOTIVOS} erro={tentou ? erroMotivo : null} />
          <AreaDeTexto rotulo="Detalhe (opcional)" linhas={3} value={detalhe} onChange={setDetalhe} />
          <p className="t13 mute" style={{ margin: 0 }}>
            O bloqueio vale depois da aprovação da coordenação e fica no lastro com o motivo. Os pacientes já agendados não são
            cancelados: a recepção remarca com cada um.
          </p>
          {op.erro !== null && <ErroDoAto erro={op.erro} ato="enviar o pedido de bloqueio" />}
          <div className="row" style={{ gap: 8 }}>
            <Botao type="submit" variante="principal" ocupado={op.enviando}>
              Enviar para aprovação
            </Botao>
            <Botao onPress={aoFechar}>Cancelar</Botao>
          </div>
        </form>
      )}
    </Gaveta>
  );
}
