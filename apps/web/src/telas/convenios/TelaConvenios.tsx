import { useQuery } from '@tanstack/react-query';
import { Botao, Chip, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { CHAVE_CONVENIOS, SITUACAO } from './comum';
import { DetalheDoConvenio } from './DetalheDoConvenio';
import { FormularioDoConvenio } from './FormularioDoConvenio';

type Painel = { tipo: 'convenio'; id: string } | { tipo: 'cadastrar' } | { tipo: 'mudar'; id: string } | null;

/**
 * Convênios e tabelas (Convenios-Tabelas.dc.html). O faturamento cadastra o contrato, as regras de
 * autorização e propõe a tabela nova; a gerência acompanha e aprova a proposta (outra pessoa, com a
 * senha de novo). O servidor usa a versão da tabela vigente na data do atendimento.
 */
export function TelaConvenios() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const lista = useQuery({ queryKey: CHAVE_CONVENIOS, queryFn: () => api.convenios.listar() });
  const [painel, setPainel] = useState<Painel>(null);
  const perfis = sessao.data?.usuario.perfis ?? [];
  const faturista = perfis.includes('faturista');
  const gerencia = perfis.includes('gerente');
  const idAberto = painel !== null && painel.tipo !== 'cadastrar' ? painel.id : (lista.data?.[0]?.id ?? null);
  const detalhe = useQuery({
    queryKey: [...CHAVE_CONVENIOS, idAberto],
    queryFn: () => api.convenios.obter(idAberto ?? ''),
    enabled: idAberto !== null,
  });
  const conjuntos = useQuery({
    queryKey: ['faturamento', 'auditoria'],
    queryFn: () => api.faturamento.auditoria(),
    enabled: painel?.tipo === 'cadastrar',
  });
  const c = detalhe.data;
  return (
    <>
      <CabecalhoDaTela titulo="Convênios" complemento={<span className="t14 mute">O servidor usa a versão da tabela vigente na data do atendimento</span>}>
        {faturista && (
          <Botao
            pequeno
            variante="principal"
            onPress={() => {
              setPainel({ tipo: 'cadastrar' });
            }}
          >
            Cadastrar operadora
          </Botao>
        )}
      </CabecalhoDaTela>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: '300px minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Operadoras" style={{ padding: 0 }}>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="os convênios"
              carregando={lista.isPending}
              erro={lista.error}
              aoTentarDeNovo={() => void lista.refetch()}
              semPermissao={{ titulo: 'Você não consulta os convênios', detalhe: 'Convênios e tabelas são do faturamento e da gerência.' }}
            />
            {lista.data?.length === 0 && <Vazio titulo="Nenhum convênio cadastrado." detalhe="O faturamento cadastra a operadora com o contrato." />}
            <ul style={{ listStyle: 'none', margin: 0, padding: '8px' }}>
              {lista.data?.map((k) => (
                <li key={k.id}>
                  <button
                    type="button"
                    className="lp"
                    aria-current={idAberto === k.id ? 'true' : undefined}
                    onClick={() => {
                      setPainel({ tipo: 'convenio', id: k.id });
                    }}
                  >
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <span className="t clamp1" style={{ display: 'block' }}>
                        {k.operadora}
                      </span>
                      <span className="s clamp1" style={{ display: 'block' }}>
                        ANS <span className="num">{k.registro_ans}</span>, {typeof k.tabela_versao === 'number' ? `tabela v${String(k.tabela_versao)}` : 'sem tabela vigente'}
                        {k.proposta_pendente ? ', proposta aguardando' : ''}
                      </span>
                    </span>
                    <Chip cor={SITUACAO[k.situacao].cor} texto={SITUACAO[k.situacao].texto} />
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do convênio">
          <div className="scroll col" style={{ gap: 14 }}>
            {painel?.tipo === 'cadastrar' && (
              <FormularioDoConvenio
                atual={null}
                disponiveis={conjuntos.data?.conjuntos ?? []}
                aoConcluir={(id) => {
                  setPainel({ tipo: 'convenio', id });
                }}
                aoFechar={() => {
                  setPainel(null);
                }}
              />
            )}
            {painel?.tipo === 'mudar' && c !== undefined && (
              <FormularioDoConvenio
                atual={c}
                disponiveis={c.auditoria.disponiveis}
                aoConcluir={(id) => {
                  setPainel({ tipo: 'convenio', id });
                }}
                aoFechar={() => {
                  setPainel({ tipo: 'convenio', id: c.id });
                }}
              />
            )}
            {painel?.tipo !== 'cadastrar' && painel?.tipo !== 'mudar' && (
              <>
                {idAberto !== null && <EstadoDaConsulta oQue="o convênio" carregando={detalhe.isPending} erro={detalhe.error} aoTentarDeNovo={() => void detalhe.refetch()} />}
                {c !== undefined && (
                  <DetalheDoConvenio
                    key={c.id}
                    c={c}
                    faturista={faturista}
                    gerencia={gerencia}
                    eu={sessao.data?.usuario.id ?? ''}
                    aoMudarContrato={() => {
                      setPainel({ tipo: 'mudar', id: c.id });
                    }}
                  />
                )}
                {idAberto === null && lista.data !== undefined && <Vazio titulo="Escolha uma operadora." detalhe="Contrato, tabela de preços, autorização e auditoria aparecem aqui." />}
              </>
            )}
          </div>
        </section>
      </div>
    </>
  );
}
