import { useQuery } from '@tanstack/react-query';
import { getRouteApi, Link, useNavigate } from '@tanstack/react-router';
import type { AgendamentoDoPaciente, PacienteResumo } from '@vidacor/api-client';
import { Abas, BarraDoPaciente, Chip, formatarData, formatarDataHora, ListaDeDados, Vazio } from '@vidacor/design-system';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { EscolhaDePaciente } from '../../componentes/EscolhaDePaciente';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { ComunicacaoDoPaciente } from '../recepcao/ComunicacaoDoPaciente';
import { ConviteDoPortal } from '../recepcao/ConviteDoPortal';
import { ListaDePacientes } from './ListaDePacientes';

const rota = getRouteApi('/app/pacientes');

const SITUACAO: Record<string, { texto: string; cor: 'verde' | 'ambar' | 'cinza' | 'rosa' | 'azul' }> = {
  agendado: { texto: 'Agendado', cor: 'azul' },
  confirmado: { texto: 'Confirmado', cor: 'verde' },
  reserva_provisoria: { texto: 'Reserva sem guia', cor: 'ambar' },
  em_atendimento: { texto: 'Em atendimento', cor: 'azul' },
  concluido: { texto: 'Realizado', cor: 'verde' },
  faltou: { texto: 'Faltou', cor: 'rosa' },
  cancelado: { texto: 'Cancelado', cor: 'cinza' },
  expirada: { texto: 'Reserva expirada', cor: 'cinza' },
};

function Linhas({ itens, vazio }: { itens: AgendamentoDoPaciente[]; vazio: string }) {
  if (itens.length === 0) return <Vazio titulo={vazio} />;
  return (
    <ul className="col" style={{ gap: 0, listStyle: 'none', margin: 0, padding: 0 }}>
      {itens.map((a) => {
        const s = SITUACAO[a.situacao] ?? { texto: a.situacao, cor: 'cinza' as const };
        return (
          <li key={a.id} className="row" style={{ gap: 10, padding: '8px 0', borderBottom: '1px solid var(--line)', alignItems: 'flex-start' }}>
            <span className="num t13" style={{ width: 120, flexShrink: 0 }}>
              {formatarDataHora(a.inicio, FUSO_DA_CLINICA)}
            </span>
            <span style={{ flex: 1, minWidth: 0 }}>
              <b style={{ display: 'block' }}>{a.exame}</b>
              <span className="t13 mute">
                {a.sala}
                {a.medico === null || a.medico === undefined ? '' : `, ${a.medico}`}
              </span>
            </span>
            <Chip cor={s.cor} texto={s.texto} />
          </li>
        );
      })}
    </ul>
  );
}

function Visao({ id }: { id: string }) {
  const { api } = useDependencias();
  const v = useQuery({ queryKey: ['paciente-visao', id], queryFn: () => api.pacientes.visao(id) });
  if (v.data === undefined) {
    return <EstadoDaConsulta oQue="o paciente" carregando={v.isPending} erro={v.error} aoTentarDeNovo={() => void v.refetch()} />;
  }
  const { identificacao: p, proximos, historico, faltas, exames_externos } = v.data;
  const alergia = p.alergias.length > 0 ? p.alergias.join(', ') : undefined;
  return (
    <>
      <BarraDoPaciente
        nome={p.nome}
        nascimento={formatarData(p.nascimento)}
        complementoDoNascimento={p.sexo === 'F' ? 'feminino' : p.sexo === 'M' ? 'masculino' : undefined}
        alergia={alergia}
        resumo={`Prontuário ${p.codigo}`}
        acoes={
          <Link to="/agendamentos/novo" className="btn btn-sm">
            Agendar exame
          </Link>
        }
      />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.4fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Exames do paciente">
          <div className="scroll">
            <Abas
              rotulo="Exames do paciente"
              abas={[
                { id: 'proximos', rotulo: `Próximos (${String(proximos.length)})`, conteudo: <Linhas itens={proximos} vazio="Nenhum agendamento futuro." /> },
                {
                  id: 'historico',
                  rotulo: `Histórico (${String(historico.length)})`,
                  conteudo: (
                    <>
                      <p className="t13 mute" style={{ margin: '0 0 8px' }}>
                        {faltas === 0 ? 'Nenhuma falta.' : `${String(faltas)} falta(s).`} {exames_externos > 0 ? `${String(exames_externos)} exame(s) externo(s) importado(s).` : ''}
                      </p>
                      <Linhas itens={historico} vazio="Nenhum exame anterior." />
                    </>
                  ),
                },
              ]}
            />
          </div>
        </section>
        <section className="card pane" aria-label="Cadastro e contato">
          <div className="scroll col" style={{ gap: 12 }}>
            <h2 className="t15 b" style={{ margin: 0 }}>
              Identificação
            </h2>
            <ListaDeDados
              larguraDoRotulo={110}
              itens={[
                { rotulo: 'Nome', valor: p.nome },
                ...(p.nome_social === null || p.nome_social === undefined ? [] : [{ rotulo: 'Nome social', valor: p.nome_social }]),
                { rotulo: 'Nascimento', valor: <b className="num">{formatarData(p.nascimento)}</b> },
                { rotulo: 'CPF', valor: <span className="num">{p.cpf_mascarado ?? 'Não informado'}</span> },
                { rotulo: 'Celular', valor: <span className="num">{p.celular ?? 'Não informado'}</span> },
                { rotulo: 'E-mail', valor: <span style={{ overflowWrap: 'anywhere' }}>{p.email ?? 'Não informado'}</span> },
                { rotulo: 'Alergias', valor: p.alergias.length > 0 ? p.alergias.join(', ') : p.nega_alergias === true ? 'Nega alergias' : 'Não perguntado' },
              ]}
            />
            <h2 className="t15 b" style={{ margin: 0 }}>
              Comunicação
            </h2>
            <ComunicacaoDoPaciente pacienteId={p.id} />
            <h2 className="t15 b" style={{ margin: 0 }}>
              Portal Meu VidaCor
            </h2>
            <p className="t13 mute" style={{ margin: 0 }}>
              Laudos aparecem no portal só depois de assinados.
            </p>
            <ConviteDoPortal pacienteId={p.id} nome={p.nome} />
            <p className="t13 mute" style={{ margin: 0 }}>
              Toda leitura de dado clínico fica registrada.
            </p>
          </div>
        </section>
      </div>
    </>
  );
}

/**
 * Pacientes: sem paciente escolhido, a lista (Pacientes-Lista.dc.html); com `?id=`, a ficha do
 * Paciente 360 (Paciente.dc.html) com a busca no alto para trocar de paciente.
 */
export function TelaPaciente() {
  const busca = rota.useSearch();
  const navegar = useNavigate();
  if (busca.id === undefined) return <ListaDePacientes />;
  return (
    <>
      <CabecalhoDaTela titulo="Paciente">
        <Link to="/pacientes" className="btn btn-sm">
          Voltar à lista
        </Link>
      </CabecalhoDaTela>
      <div style={{ maxWidth: 520 }}>
        <EscolhaDePaciente
          rotulo="Buscar paciente"
          paciente={null}
          aoEscolher={(p: PacienteResumo | null) => {
            if (p !== null) void navegar({ to: '/pacientes', search: { id: p.id } });
          }}
        />
      </div>
      <Visao key={busca.id} id={busca.id} />
    </>
  );
}
