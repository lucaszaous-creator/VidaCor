import { useQuery } from '@tanstack/react-query';
import { Busca, Chip, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { useDependencias } from '../../contexto';
import { CHAVE_CORPO_CLINICO, resumoDoRegistro, SITUACAO_DO_MEDICO } from './comum';
import { DetalheDoMedico } from './DetalheDoMedico';

function normalizar(t: string): string {
  return t
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLocaleLowerCase('pt-BR');
}

/**
 * Corpo clínico (Corpo-Clinico.dc.html). Para a gerência: médicos da unidade com CRM, RQE e
 * especialidade e o que cada um realiza e lauda. Toda mudança vai para a conferência de outra pessoa
 * da gerência e só vale depois dela; o servidor confere e pede a senha de novo (docs/04, docs/18).
 */
export function TelaCorpoClinico() {
  const { api } = useDependencias();
  const lista = useQuery({
    queryKey: CHAVE_CORPO_CLINICO,
    queryFn: () => api.corpoClinico.listar(),
  });
  const [busca, setBusca] = useState('');
  const [escolhido, setEscolhido] = useState<string | null>(null);
  const medicos = (lista.data ?? []).filter((m) => busca.trim() === '' || normalizar(`${m.nome} ${m.registro?.numero ?? ''}`).includes(normalizar(busca.trim())));
  const atual = lista.data?.find((m) => m.id === escolhido) ?? null;
  const aguardando = (lista.data ?? []).filter((m) => m.pendentes.length > 0).length;
  return (
    <>
      <CabecalhoDaTela
        titulo="Corpo clínico"
        complemento={
          lista.data === undefined ? undefined : (
            <span className="t14 mute">
              {lista.data.length} médicos na unidade, {aguardando} aguardando conferência
            </span>
          )
        }
      />
      <div
        className="fill"
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.3fr)',
          gridTemplateRows: 'minmax(0, 1fr)',
          gap: 16,
        }}
      >
        <section className="card pane" aria-label="Médicos da unidade" style={{ padding: 0 }}>
          <div style={{ padding: '14px 16px 8px' }}>
            <Busca rotulo="Buscar médico" valor={busca} aoMudar={setBusca} placeholder="Nome ou número do CRM" />
          </div>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="o corpo clínico"
              carregando={lista.isPending}
              erro={lista.error}
              aoTentarDeNovo={() => void lista.refetch()}
              semPermissao={{
                titulo: 'Você não administra o corpo clínico',
                detalhe: 'O cadastro dos médicos é da gerência.',
              }}
            />
            {lista.data !== undefined && medicos.length === 0 && <Vazio titulo="Nenhum médico encontrado." detalhe="Busque por parte do nome ou pelo número do CRM." />}
            <ul style={{ listStyle: 'none', margin: 0, padding: '0 8px 8px' }}>
              {medicos.map((m) => (
                <li key={m.id}>
                  <button
                    type="button"
                    className="lp"
                    aria-current={m.id === escolhido ? 'true' : undefined}
                    onClick={() => {
                      setEscolhido(m.id);
                    }}
                  >
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <span className="t clamp1" style={{ display: 'block' }}>
                        {m.nome}
                      </span>
                      <span className="s clamp1" style={{ display: 'block' }}>
                        {resumoDoRegistro(m)}
                      </span>
                    </span>
                    <Chip cor={SITUACAO_DO_MEDICO[m.situacao].cor} texto={SITUACAO_DO_MEDICO[m.situacao].texto} />
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do médico">
          <div className="scroll col" style={{ gap: 14 }}>
            {atual === null ? (
              <Vazio titulo="Escolha um médico." detalhe="Registro profissional, o que realiza e lauda e o histórico aparecem aqui." />
            ) : (
              <DetalheDoMedico key={atual.id} m={atual} />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
