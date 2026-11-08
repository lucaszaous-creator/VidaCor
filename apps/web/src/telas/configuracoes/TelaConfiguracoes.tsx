import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { IntegracaoVista } from '@vidacor/api-client';
import { Aviso, Chip, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { useDependencias } from '../../contexto';

const SITUACAO: Record<IntegracaoVista['situacao'], { texto: string; cor: 'verde' | 'ambar' | 'cinza' | 'azul' | 'rosa' }> = {
  em_uso: { texto: 'Em uso', cor: 'verde' },
  desenvolvimento: { texto: 'Simulado de desenvolvimento', cor: 'azul' },
  incompleta: { texto: 'Incompleta', cor: 'ambar' },
  aguardando_adaptador: { texto: 'Aguardando adaptador', cor: 'ambar' },
  nao_configurada: { texto: 'Não configurada', cor: 'cinza' },
};

const OUTRAS = [
  { href: '/usuarios', rotulo: 'Usuários e perfis', detalhe: 'Contas, perfis e pedidos aguardando a segunda pessoa.' },
  { href: '/aparelhos', rotulo: 'Aparelhos', detalhe: 'Cadastro, desativação e atividade do dia dos aparelhos DICOM.' },
  { href: '/lista-espera', rotulo: 'Horários e bloqueios', detalhe: 'Bloqueio de sala por feriado ou manutenção.' },
  { href: '/lastro', rotulo: 'Log de auditoria', detalhe: 'Cadeia de eventos de qualquer código, com hash.' },
] as const;

function Detalhe({ i }: { i: IntegracaoVista }) {
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {i.nome}
        </h2>
        <Chip cor={SITUACAO[i.situacao].cor} texto={SITUACAO[i.situacao].texto} />
      </div>
      <p className="t14" style={{ margin: 0 }}>
        {i.para_que}
      </p>
      {i.o_que_falta !== null && i.o_que_falta !== undefined && (
        <Aviso nivel={i.situacao === 'desenvolvimento' ? 'informativo' : 'atencao'} titulo={SITUACAO[i.situacao].texto + '.'}>
          {i.o_que_falta}
        </Aviso>
      )}
      {i.pendentes_no_outbox > 0 && (
        <Aviso nivel="informativo" titulo={`${String(i.pendentes_no_outbox)} envio(s) aguardando esta integração.`}>
          Nada se perde: eles saem sozinhos quando a integração estiver pronta.
        </Aviso>
      )}
      {i.variaveis.length > 0 && (
        <>
          <h3 className="t14 b" style={{ margin: 0 }}>
            Variáveis no ambiente do servidor
          </h3>
          <p className="t13 mute" style={{ margin: 0 }}>
            Os valores ficam só no servidor e não aparecem aqui. A TI preenche no arquivo de ambiente (modelo em infra/compose/.env.exemplo) e reinicia o núcleo.
          </p>
          <ul className="col" style={{ gap: 8, listStyle: 'none', margin: 0, padding: 0 }}>
            {i.variaveis.map((x) => (
              <li key={x.nome} className="row" style={{ gap: 10, alignItems: 'flex-start', borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <code className="t13" style={{ overflowWrap: 'anywhere' }}>
                    {x.nome}
                  </code>
                  <div className="t13 mute">
                    {x.descricao}
                    {x.obrigatoria ? '' : ' Opcional.'}
                    {x.segredo ? ' Segredo.' : ''}
                  </div>
                </div>
                <Chip cor={x.preenchida ? 'verde' : x.obrigatoria ? 'ambar' : 'cinza'} texto={x.preenchida ? 'Preenchida' : 'Vazia'} />
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

/**
 * Configurações da unidade (Configuracoes.dc.html): integrações externas com o que falta para cada uma
 * e o acesso às demais configurações. Toda mudança gera versão e fica no lastro nas telas próprias.
 */
export function TelaConfiguracoes() {
  const { api } = useDependencias();
  const lista = useQuery({ queryKey: ['integracoes'], queryFn: () => api.integracoes() });
  const [escolhida, setEscolhida] = useState<string>('email');
  const atual = lista.data?.find((i) => i.id === escolhida) ?? null;
  return (
    <>
      <CabecalhoDaTela titulo="Configurações" complemento={<span className="t14 mute">Toda mudança gera versão e fica no lastro.</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.3fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <div className="col" style={{ gap: 16, minHeight: 0 }}>
          <section className="card pane" aria-label="Integrações" style={{ padding: 0, flex: 1, minHeight: 0 }}>
            <div className="card-h" style={{ padding: '14px 16px 0', marginBottom: 8 }}>
              <h2>Integrações</h2>
            </div>
            <div className="scroll">
              <EstadoDaConsulta oQue="as integrações" carregando={lista.isPending} erro={lista.error} aoTentarDeNovo={() => void lista.refetch()} />
              <ul style={{ listStyle: 'none', margin: 0, padding: '0 8px 8px' }}>
                {lista.data?.map((i) => (
                  <li key={i.id}>
                    <button
                      type="button"
                      aria-current={i.id === escolhida ? 'true' : undefined}
                      style={{ width: '100%', textAlign: 'left', display: 'flex', gap: 10, alignItems: 'center', padding: '10px 8px', border: 0, background: i.id === escolhida ? 'var(--line)' : 'transparent', borderBottom: '1px solid var(--line)', cursor: 'pointer', font: 'inherit', color: 'inherit' }}
                      onClick={() => {
                        setEscolhida(i.id);
                      }}
                    >
                      <span style={{ flex: 1, minWidth: 0 }}>
                        <b style={{ display: 'block' }}>{i.nome}</b>
                        <span className="t13 mute">{i.para_que}</span>
                      </span>
                      <Chip cor={SITUACAO[i.situacao].cor} texto={SITUACAO[i.situacao].texto} />
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          </section>
          <nav className="card pane" aria-label="Outras configurações">
            <h2 className="t15 b" style={{ margin: '0 0 8px' }}>
              Outras configurações
            </h2>
            <ul className="col" style={{ gap: 6, listStyle: 'none', margin: 0, padding: 0 }}>
              {OUTRAS.map((o) => (
                <li key={o.href}>
                  <Link to={o.href} className="b">
                    {o.rotulo}
                  </Link>
                  <span className="t13 mute">, {o.detalhe.charAt(0).toLowerCase() + o.detalhe.slice(1)}</span>
                </li>
              ))}
            </ul>
          </nav>
        </div>
        <section className="card pane" aria-label="Detalhe da integração">
          <div className="scroll">{atual === null ? <Vazio titulo="Escolha uma integração." /> : <Detalhe i={atual} />}</div>
        </section>
      </div>
    </>
  );
}
