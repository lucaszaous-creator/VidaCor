import { useQuery } from '@tanstack/react-query';
import type { ExameComDose } from '@vidacor/api-client';
import { Abas, Botao, Chip, formatarData, formatarDataHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { HistoricoDoPaciente, JustificarDose } from './PainelDaDose';
import { numero, UNIDADE } from './formatos';
import { ProtocolosENrd } from './ProtocolosENrd';


function valores(e: ExameComDose): string {
  const partes: string[] = [];
  if (typeof e.ctdivol_mgy === 'number') partes.push(`CTDIvol ${numero(e.ctdivol_mgy)} mGy`);
  if (typeof e.dlp_mgycm === 'number') partes.push(`DLP ${numero(e.dlp_mgycm, 0)} mGy·cm`);
  if (typeof e.dap_gycm2 === 'number') partes.push(`DAP ${numero(e.dap_gycm2, 2)} Gy·cm²`);
  return partes.join(', ');
}

function comparacao(e: ExameComDose): string {
  if (e.nrd === null || e.nrd === undefined || typeof e.valor_comparado !== 'number') {
    return 'Sem NRD cadastrado para este exame';
  }
  const pct = Math.round((e.valor_comparado / e.nrd.valor) * 100 - 100);
  const nrd = `NRD ${numero(e.nrd.valor)} ${UNIDADE[e.nrd.grandeza]}`;
  return e.acima_do_nrd ? `${nrd}, ${String(pct)}% acima` : `${nrd}, dentro`;
}

function Exames({ exames, selecionado, aoSelecionar, aoJustificar }: { exames: ExameComDose[]; selecionado: string | null; aoSelecionar: (e: ExameComDose) => void; aoJustificar: (e: ExameComDose) => void }) {
  if (exames.length === 0) {
    return <Vazio titulo="Nenhum exame com dose no período entre os seus pacientes." />;
  }
  return (
    <table className="tb sticky" aria-label="Exames com dose">
      <thead>
        <tr>
          <th>Paciente</th>
          <th>Exame</th>
          <th>Dose</th>
          <th>Comparação com NRD</th>
        </tr>
      </thead>
      <tbody>
        {exames.map((e) => (
          <tr
            key={e.dose_id}
            className={selecionado === e.dose_id ? 'sel' : undefined}
            aria-selected={selecionado === e.dose_id}
            tabIndex={0}
            onClick={() => {
              aoSelecionar(e);
            }}
            onKeyDown={(ev) => {
              if (ev.key === 'Enter') aoSelecionar(e);
            }}
          >
            <td>
              <b style={{ display: 'block' }}>{e.paciente.nome}</b>
              <span className="t13 mute">nascimento {formatarData(e.paciente.nascimento)}</span>
              {e.menor_de_idade && (
                <div>
                  <Chip cor="ambar" texto="Menor de 18 anos" />
                </div>
              )}
            </td>
            <td>
              <span style={{ display: 'block' }}>{e.exame}</span>
              <span className="t13 mute">
                {e.aparelho ?? e.modalidade}, {formatarDataHora(e.realizado_em, FUSO_DA_CLINICA)}
              </span>
            </td>
            <td className="num">{valores(e)}</td>
            <td>
              <Chip
                cor={!e.acima_do_nrd ? 'verde' : e.justificativa === null || e.justificativa === undefined ? 'rosa' : 'azul'}
                texto={!e.acima_do_nrd ? 'Dentro do NRD' : e.justificativa === null || e.justificativa === undefined ? 'Acima do NRD' : 'Acima, justificado'}
              />
              <span className="t13 mute" style={{ display: 'block' }}>
                {comparacao(e)}
              </span>
              {e.acima_do_nrd && (e.justificativa === null || e.justificativa === undefined) && (
                <Botao
                  pequeno
                  onPress={() => {
                    aoJustificar(e);
                  }}
                >
                  Justificar
                </Botao>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function ExamesDoMedico() {
  const { api } = useDependencias();
  const q = useQuery({ queryKey: ['doses'], queryFn: () => api.doses.listar() });
  const [selecionado, setSelecionado] = useState<ExameComDose | null>(null);
  const [justificar, setJustificar] = useState<ExameComDose | null>(null);
  if (q.data === undefined) {
    return <EstadoDaConsulta oQue="os exames com dose" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} />;
  }
  const d = q.data;
  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(260px, 320px)', gap: 16, height: 'calc(100vh - 260px)', minHeight: 300 }}>
      <div className="col" style={{ minHeight: 0, gap: 8 }}>
        <p className="t14 mute" style={{ margin: 0 }}>
          De {formatarData(d.de)} a {formatarData(d.ate)}: {d.exames.length} exames com dose, {d.acima_do_nrd} acima do NRD, {d.sem_justificativa} sem justificativa.
        </p>
        <div style={{ overflowY: 'auto', minHeight: 0, flex: 1 }}>
          <Exames exames={d.exames} selecionado={selecionado?.dose_id ?? null} aoSelecionar={setSelecionado} aoJustificar={setJustificar} />
        </div>
      </div>
      <aside className="col" aria-label="Paciente" style={{ minHeight: 0, overflowY: 'auto' }}>
        {selecionado === null ? <Vazio titulo="Escolha um exame para ver o histórico de dose do paciente." /> : <HistoricoDoPaciente pacienteId={selecionado.paciente.id} />}
      </aside>
      {justificar !== null && (
        <JustificarDose
          exame={justificar}
          aoFechar={() => {
            setJustificar(null);
            void q.refetch();
          }}
        />
      )}
    </div>
  );
}

/**
 * Gestão de dose (Gestao-Dose.dc.html): exames de TC e raio X com a dose lida do RDSR e a comparação
 * com o NRD. O médico vê os pacientes com quem tem vínculo de atendimento e justifica a dose acima do
 * NRD; a gerência vê o agregado por protocolo e publica o NRD. O servidor decide o que cada um vê.
 */
export function TelaGestaoDeDose() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const medico = sessao.data?.usuario.perfis.includes('medico') ?? false;
  const abas = [
    ...(medico ? [{ id: 'exames', rotulo: 'Exames', conteudo: <ExamesDoMedico /> }] : []),
    { id: 'protocolos', rotulo: 'Protocolos e NRD', conteudo: <ProtocolosENrd /> },
  ];
  return (
    <>
      <CabecalhoDaTela
        titulo="Gestão de dose"
        complemento={<span className="t14 mute">Doses lidas do RDSR de cada aparelho e comparadas ao nível de referência (NRD) do exame.</span>}
      />
      <section className="card pane fill" aria-label="Gestão de dose" style={{ minHeight: 0 }}>
        {sessao.data === undefined ? null : <Abas rotulo="Gestão de dose" abas={abas} />}
      </section>
    </>
  );
}
