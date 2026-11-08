import { useQuery } from '@tanstack/react-query';
import { getRouteApi, useRouterState } from '@tanstack/react-router';
import { Busca, formatarDiaPorExtenso, formatarHora, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { consultaAgenda } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useRetomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA, hojeNaClinica } from '../agenda/grade';
import { aparenciaDoAgendamento } from '../agenda/situacoes';
import { PainelDoPaciente } from './PainelDoPaciente';

const rota = getRouteApi('/app/recepcao');

function normalizar(texto: string): string {
  return texto.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLocaleLowerCase('pt-BR');
}

/**
 * Recepção (Recepcao.dc.html e Recepcao-Interativa.dc.html): pacientes do dia à esquerda, o paciente
 * escolhido à direita com check-in, senha, pulseira, reimpressão e correção de cadastro.
 */
export function TelaRecepcao() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const data = busca.data ?? hojeNaClinica();
  const retomada = useRetomada();
  const agendamentoDaRetomada = useRouterState({ select: (s) => (s.location.search as Record<string, unknown>).agendamento });
  const [selecionado, setSelecionado] = useState<string | null>(() =>
    typeof agendamentoDaRetomada === 'string' && /^[0-9a-f-]{36}$/i.test(agendamentoDaRetomada) ? agendamentoDaRetomada : null,
  );
  const [filtro, setFiltro] = useState('');
  const agenda = useQuery(consultaAgenda(api, data, busca.sala));

  const itens = [...(agenda.data?.itens ?? [])].sort((a, b) => a.inicio.localeCompare(b.inicio));
  const visiveis = filtro.trim() === '' ? itens : itens.filter((i) => normalizar(i.paciente.nome).includes(normalizar(filtro.trim())));
  const item = itens.find((i) => i.id === selecionado) ?? null;

  return (
    <>
      <CabecalhoDaTela titulo="Recepção e pulseiras" complemento={<span className="t14 mute">{formatarDiaPorExtenso(data)}</span>}>
        <div style={{ width: 320 }}>
          <Busca rotulo="Buscar paciente do dia" valor={filtro} aoMudar={setFiltro} placeholder="Buscar paciente do dia" />
        </div>
      </CabecalhoDaTela>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Pacientes do dia" style={{ padding: 0 }}>
          <div className="card-h" style={{ padding: '14px 16px 0', marginBottom: 8 }}>
            <h2>Pacientes do dia</h2>
            <span className="t13 mute">{agenda.data === undefined ? '' : `${String(itens.length)} agendados`}</span>
          </div>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="os pacientes do dia"
              carregando={agenda.isPending}
              erro={agenda.error}
              aoTentarDeNovo={() => void agenda.refetch()}
              semPermissao={{ titulo: 'Você não tem acesso à recepção', detalhe: 'Peça à coordenação o perfil da recepção.' }}
            />
            {agenda.data !== undefined && visiveis.length === 0 && (
              <div style={{ padding: 16 }}>
                <Vazio
                  titulo={filtro.trim() === '' ? 'Nenhum paciente agendado neste dia' : 'Nenhum paciente com esse nome hoje'}
                  detalhe={filtro.trim() === '' ? undefined : 'Confira a grafia ou busque pelo sobrenome.'}
                />
              </div>
            )}
            {visiveis.map((i) => {
              const ap = aparenciaDoAgendamento(i.situacao);
              return (
                <LinhaDoPaciente
                  key={i.id}
                  nome={i.paciente.nome}
                  nascimento={i.paciente.nascimento}
                  inicio={formatarHora(i.inicio, FUSO_DA_CLINICA)}
                  detalhe={i.exame.nome}
                  situacao={{ texto: ap.texto, cor: ap.cor }}
                  marca={i.homonimos.length > 0 ? 'Homônimo hoje: confira o nascimento' : undefined}
                  selecionada={i.id === selecionado}
                  contextoDoMenu={`${formatarHora(i.inicio, FUSO_DA_CLINICA)}, ${i.exame.nome}`}
                  menuDe={{ objeto: 'agendamento', id: i.id }}
                  aoAbrir={() => {
                    setSelecionado(i.id);
                  }}
                />
              );
            })}
          </div>
        </section>
        <section className="card pane" aria-label="Paciente escolhido">
          <div className="scroll" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
            {item === null ? (
              <Vazio titulo="Escolha um paciente da lista" detalhe="O check-in, a senha e a pulseira aparecem aqui." />
            ) : (
              <PainelDoPaciente key={item.id} item={item} retomada={retomada} />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
