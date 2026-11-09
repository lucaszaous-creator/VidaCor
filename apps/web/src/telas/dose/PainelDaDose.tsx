import { useQuery } from '@tanstack/react-query';
import type { ExameComDose, MotivoDeJustificativa, NovaJustificativa } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, Escolha, formatarData, Gaveta, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { numero } from './formatos';

export const NOME_DA_JUSTIFICATIVA: Record<MotivoDeJustificativa, string> = {
  imc_alto: 'Paciente com IMC alto',
  repeticao_movimento: 'Repetição por movimento',
  fase_extra: 'Fase extra pedida pelo médico',
  implante_metalico: 'Implante metálico',
  outro: 'Outro',
};

/** Histórico de dose do paciente nos últimos 12 meses (só com vínculo; o servidor confere). */
export function HistoricoDoPaciente({ pacienteId }: { pacienteId: string }) {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['dose-paciente', pacienteId], queryFn: () => api.doses.historico(pacienteId) });
  if (q.data === undefined) {
    return <EstadoDaConsulta oQue="o histórico de dose" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} linhas={2} />;
  }
  const h = q.data;
  return (
    <div className="col" style={{ gap: 8 }}>
      <div>
        <b style={{ display: 'block' }}>{h.paciente.nome}</b>
        <span className="t13 mute">nascimento {formatarData(h.paciente.nascimento)}, {h.exames.length} exame(s) com radiação em 12 meses</span>
      </div>
      <div className="t14">
        Dose efetiva estimada em 12 meses:{' '}
        <b className="num">{typeof h.dose_efetiva_12_meses_msv === 'number' ? `${numero(h.dose_efetiva_12_meses_msv, 2)} mSv` : 'sem estimativa'}</b>
      </div>
      <p className="t13 mute" style={{ margin: 0 }}>
        Estimativa só desta clínica, para orientar o médico; não é dosimetria individual.
        {h.exames_sem_estimativa > 0 ? ` ${String(h.exames_sem_estimativa)} exame(s) sem fator cadastrado no NRD ficaram fora da soma.` : ''}
      </p>
      {h.exames.length === 0 ? (
        <Vazio titulo="Sem exames com dose em 12 meses." />
      ) : (
        <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }}>
          {h.exames.map((e) => (
            <li key={`${e.exame}-${e.realizado_em}`} className="row" style={{ padding: '6px 0', borderBottom: '1px solid var(--line)', alignItems: 'flex-start', gap: 8 }}>
              <span style={{ flex: 1, minWidth: 0 }}>
                <span style={{ display: 'block' }}>
                  {e.exame}, {formatarData(e.realizado_em.slice(0, 10))}
                </span>
                <span className="t13 mute">
                  {e.aparelho ?? ''}
                  {typeof e.dlp_mgycm === 'number' ? `, DLP ${numero(e.dlp_mgycm, 0)}` : ''}
                  {typeof e.dap_gycm2 === 'number' ? `, DAP ${numero(e.dap_gycm2, 2)}` : ''}
                </span>
              </span>
              <span className="num">{typeof e.dose_efetiva_msv === 'number' ? `${numero(e.dose_efetiva_msv, 2)} mSv` : 'sem estimativa'}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

const MOTIVOS = (Object.keys(NOME_DA_JUSTIFICATIVA) as MotivoDeJustificativa[]).map((m) => ({ valor: m, rotulo: NOME_DA_JUSTIFICATIVA[m] }));

/** Justificativa da dose acima do NRD (médico com vínculo). Fica no lastro do estudo. */
export function JustificarDose({ exame, aoFechar }: { exame: ExameComDose; aoFechar: () => void }) {
  const { api } = useDependencias();
  const [motivo, setMotivo] = useState('');
  const [detalhe, setDetalhe] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovaJustificativa, chave: string) => api.doses.justificar(exame.dose_id, n, { idempotencyKey: chave }));
  const erroMotivo = motivo === '' ? 'Escolha o motivo.' : null;
  const erroDetalhe = motivo === 'outro' && detalhe.trim().length < 5 ? 'Em "Outro", descreva o motivo (5 letras ou mais).' : null;
  return (
    <Gaveta titulo="Justificar dose acima do NRD" subTitulo={`${exame.paciente.nome}, nascimento ${formatarData(exame.paciente.nascimento)}`} rotulo="Justificativa de dose" aoFechar={aoFechar}>
      {op.resultado !== null ? (
        <div className="col" style={{ gap: 10 }}>
          <Aviso nivel="feito" titulo="Justificativa registrada.">
            Anexada ao estudo de {exame.paciente.nome} e registrada no lastro. Entra na revisão periódica da proteção radiológica.
          </Aviso>
          <div className="row">
            <Botao onPress={aoFechar}>Fechar</Botao>
          </div>
        </div>
      ) : (
        <form
          className="col"
          style={{ gap: 10 }}
          aria-label="Justificativa de dose"
          noValidate
          onSubmit={(ev) => {
            ev.preventDefault();
            setTentou(true);
            if (erroMotivo === null && erroDetalhe === null)
              op.enviar({ motivo: motivo as MotivoDeJustificativa, detalhe: detalhe.trim() === '' ? null : detalhe.trim() });
          }}
        >
          <p className="t14" style={{ margin: 0 }}>
            {exame.exame}. A justificativa não muda depois de registrada.
          </p>
          <Escolha rotulo="Motivo" valor={motivo} aoMudar={setMotivo} vazio="Escolha o motivo" opcoes={MOTIVOS} erro={tentou ? erroMotivo : null} />
          <AreaDeTexto rotulo="Detalhe" linhas={3} value={detalhe} onChange={setDetalhe} erro={tentou ? erroDetalhe : null} />
          {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar a justificativa" />}
          <div className="row" style={{ gap: 8 }}>
            <Botao type="submit" variante="principal" ocupado={op.enviando}>
              Registrar justificativa
            </Botao>
            <Botao onPress={aoFechar}>Agora não</Botao>
          </div>
        </form>
      )}
    </Gaveta>
  );
}
