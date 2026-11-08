import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Botao, Campo, Chip, formatarDataHora } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const MODELOS: Record<string, string> = { confirmacao: 'Confirmação de horário', laudo_pronto: 'Resultado no portal', reserva_expirada: 'Reserva expirada' };
const SITUACOES: Record<string, string> = { enviada: 'Enviada', bloqueada_opt_out: 'Não enviada: paciente desligou', sem_celular: 'Não enviada: sem celular' };

/**
 * WhatsApp do paciente na recepção: se está ligado, o histórico de mensagens e a troca com motivo.
 * As mensagens não levam dado de saúde; "SAIR" pelo próprio WhatsApp também desliga.
 */
export function ComunicacaoDoPaciente({ pacienteId }: { pacienteId: string }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const chave = ['comunicacao', pacienteId];
  const q = useQuery({ queryKey: chave, queryFn: () => api.comunicacao.doPaciente(pacienteId) });
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((ligar: boolean) => api.comunicacao.preferir(pacienteId, { whatsapp: ligar, motivo: motivo.trim() }), {
    aoConcluir: (r) => {
      cliente.setQueryData(chave, r);
      setMotivo('');
      setTentou(false);
    },
  });
  if (q.data === undefined) return null;
  const c = q.data;
  return (
    <section className="col" style={{ gap: 8 }} aria-label="WhatsApp do paciente">
      <div className="row" style={{ gap: 8 }}>
        <h3 className="t15 b" style={{ margin: 0, flex: 1 }}>
          WhatsApp
        </h3>
        {c.whatsapp ? <Chip cor="verde" texto="Avisos ligados" /> : <Chip cor="ambar" texto="Avisos desligados" />}
      </div>
      {!c.whatsapp && typeof c.motivo === 'string' && <span className="t13 mute">Motivo: {c.motivo}</span>}
      {c.mensagens.slice(0, 3).map((m) => (
        <span key={`${m.modelo}-${m.criado_em}`} className="t13">
          {MODELOS[m.modelo] ?? m.modelo}, {formatarDataHora(m.criado_em, FUSO_DA_CLINICA)}: {SITUACOES[m.situacao] ?? m.situacao}
          {m.resposta === 'confirmou' ? ', paciente confirmou' : ''}
        </span>
      ))}
      <div className="row" style={{ gap: 8, alignItems: 'flex-end' }}>
        <div style={{ flex: 1 }}>
          <Campo rotulo="Motivo da troca" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 3 ? 'Diga o motivo.' : null} />
        </div>
        <Botao
          pequeno
          ocupado={op.enviando}
          onPress={() => {
            setTentou(true);
            if (motivo.trim().length >= 3) op.enviar(!c.whatsapp);
          }}
        >
          {c.whatsapp ? 'Desligar avisos' : 'Ligar avisos'}
        </Botao>
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="trocar os avisos" />}
    </section>
  );
}
