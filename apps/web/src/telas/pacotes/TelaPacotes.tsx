import { useQuery } from '@tanstack/react-query';
import type { PacoteDeCheckup, VersaoDoPacoteDeCheckup } from '@vidacor/api-client';
import { Aviso, Botao, Chip, Escolha, formatarDataHora, formatarDinheiro, formatarPontosBase, Tabela, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CHAVE_PACOTES, dataCurta, SITUACAO_DA_VERSAO, versaoPrincipal } from './comum';
import { FormularioDoPacote } from './FormularioDoPacote';

type Painel = { tipo: 'pacote'; codigo: string } | { tipo: 'novo' } | { tipo: 'versao'; codigo: string } | null;

function Detalhe({ p, gerencia, aoNovaVersao }: { p: PacoteDeCheckup; gerencia: boolean; aoNovaVersao: () => void }) {
  const [escolhida, setEscolhida] = useState('');
  const v: VersaoDoPacoteDeCheckup | undefined = p.versoes.find((x) => String(x.regra_versao) === escolhida) ?? versaoPrincipal(p.versoes);
  if (v === undefined) return null;
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row wrap" style={{ gap: 12, alignItems: 'end' }}>
        <div style={{ flex: '1 1 260px', minWidth: 0 }}>
          <h2 style={{ margin: 0, fontSize: 18, fontWeight: 650 }}>{v.nome}</h2>
          <div className="t13 mute">{v.para_quem ?? 'Sem descrição de público'}</div>
        </div>
        {p.versoes.length > 1 && (
          <div style={{ flex: '0 1 280px', minWidth: 0 }}>
            <Escolha
              rotulo="Versão do pacote"
              valor={escolhida}
              aoMudar={setEscolhida}
              vazio="A vigente hoje"
              opcoes={p.versoes.map((x) => ({ valor: String(x.regra_versao), rotulo: `v${String(x.regra_versao)}, desde ${dataCurta(x.vigente_desde)} (${SITUACAO_DA_VERSAO[x.situacao]})` }))}
            />
          </div>
        )}
      </div>
      <Tabela
        rotulo={`Exames de ${v.nome}`}
        colunas={[
          { id: 'ordem', rotulo: 'Ordem', semQuebra: true, celula: (e) => <span className="num">{e.ordem}</span> },
          {
            id: 'exame',
            rotulo: 'Exame',
            cabecalhoDaLinha: true,
            celula: (e) => (
              <>
                <div className="b">{e.nome}</div>
                {typeof e.porque === 'string' && <div className="t13 mute">{e.porque}</div>}
              </>
            ),
          },
          { id: 'preparo', rotulo: 'Preparo', celula: (e) => e.preparo ?? 'Não informado' },
          { id: 'tempo', rotulo: 'Tempo', direita: true, semQuebra: true, celula: (e) => <span className="num">{e.duracao_min} min</span> },
          { id: 'avulso', rotulo: 'Avulso', direita: true, semQuebra: true, celula: (e) => <span className="num">{typeof e.avulso_centavos === 'number' ? formatarDinheiro(e.avulso_centavos) : 'Sem preço'}</span> },
        ]}
        linhas={v.exames.map((e) => ({ ...e, id: e.tuss }))}
      />
      {!v.vale_no_orcamento && (
        <Aviso nivel="atencao" titulo="O orçamento não aplica este pacote.">
          {typeof v.avulso_centavos === 'number'
            ? 'A soma avulsa pela tabela do particular ficou igual ou menor que o preço do pacote. Publique uma versão nova com outro preço.'
            : 'Algum exame não tem preço na tabela do particular, então o avulso não pode ser calculado.'}
        </Aviso>
      )}
      <div className="row wrap" style={{ gap: 16, borderTop: '1px solid var(--line)', paddingTop: 12 }}>
        <div>
          <div className="t13 mute">Preço do pacote</div>
          <div className="num" style={{ fontSize: 24, fontWeight: 650 }}>
            {formatarDinheiro(v.preco_centavos)}
          </div>
          <div className="t12 mute num">
            {typeof v.avulso_centavos === 'number' ? `Avulso ${formatarDinheiro(v.avulso_centavos)}` : 'Avulso indisponível'}
            {typeof v.desconto_pontos_base === 'number' ? `, ${formatarPontosBase(v.desconto_pontos_base)} a menos` : ''}
          </div>
        </div>
        <div className="t13 mute" style={{ flex: '1 1 240px', lineHeight: 1.5 }}>
          Regra de preço v{v.regra_versao}, {SITUACAO_DA_VERSAO[v.situacao]}, desde {dataCurta(v.vigente_desde)}, cerca de {v.duracao_total_min} min no dia
          {typeof v.publicada_por === 'string' ? `. Publicada por ${v.publicada_por} em ${formatarDataHora(v.publicada_em, FUSO_DA_CLINICA)}` : ''}
          {typeof v.motivo === 'string' ? `: ${v.motivo}` : ''}. Preço calculado e conferido pelo servidor.
        </div>
        {gerencia && (
          <Botao pequeno onPress={aoNovaVersao}>
            Nova versão
          </Botao>
        )}
      </div>
    </div>
  );
}

/**
 * Check-up e pacotes (Checkup-Pacotes.dc.html): vários exames num dia, na ordem certa, com o preço do
 * pacote calculado e conferido pelo servidor. A recepção consulta para orientar o paciente; a gerência
 * publica versão nova, que começa numa data futura e não muda a vigente.
 */
export function TelaPacotes() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const catalogo = useQuery({ queryKey: CHAVE_PACOTES, queryFn: () => api.pacotes.listar() });
  const [painel, setPainel] = useState<Painel>(null);
  const gerencia = sessao.data?.usuario.perfis.includes('gerente') ?? false;
  const pacotes = catalogo.data?.pacotes ?? [];
  const codigo = painel !== null && painel.tipo !== 'novo' ? painel.codigo : pacotes[0]?.codigo;
  const atual = pacotes.find((p) => p.codigo === codigo) ?? null;
  return (
    <>
      <CabecalhoDaTela titulo="Check-up e pacotes" complemento={<span className="t14 mute">Vários exames num dia, na ordem certa; preço calculado pelo servidor</span>}>
        {gerencia && (
          <Botao
            pequeno
            variante="principal"
            onPress={() => {
              setPainel({ tipo: 'novo' });
            }}
          >
            Novo pacote
          </Botao>
        )}
      </CabecalhoDaTela>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: '300px minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Pacotes" style={{ padding: 0 }}>
          <div className="scroll">
            <EstadoDaConsulta
              oQue="os pacotes"
              carregando={catalogo.isPending}
              erro={catalogo.error}
              aoTentarDeNovo={() => void catalogo.refetch()}
              semPermissao={{ titulo: 'Você não consulta os pacotes', detalhe: 'Check-up e pacotes são da recepção e da gerência.' }}
            />
            {catalogo.data !== undefined && pacotes.length === 0 && <Vazio titulo="Nenhum pacote cadastrado." detalhe="A gerência monta o pacote com os exames, a ordem e o preço." />}
            <ul style={{ listStyle: 'none', margin: 0, padding: '8px' }}>
              {pacotes.map((p) => {
                const v = versaoPrincipal(p.versoes);
                return (
                  <li key={p.codigo}>
                    <button
                      type="button"
                      className="lp"
                      aria-current={codigo === p.codigo ? 'true' : undefined}
                      onClick={() => {
                        setPainel({ tipo: 'pacote', codigo: p.codigo });
                      }}
                    >
                      <span style={{ flex: 1, minWidth: 0 }}>
                        <span className="t clamp1" style={{ display: 'block' }}>
                          {p.nome}
                        </span>
                        <span className="s clamp1" style={{ display: 'block' }}>
                          {v === undefined ? '' : `${String(v.exames.length)} exames, cerca de ${String(v.duracao_total_min)} min`}
                        </span>
                      </span>
                      {v !== undefined && <span className="num t14 b">{formatarDinheiro(v.preco_centavos)}</span>}
                      {v !== undefined && v.situacao !== 'vigente' && <Chip cor="cinza" texto="Começa depois" />}
                    </button>
                  </li>
                );
              })}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do pacote">
          <div className="scroll col" style={{ gap: 14 }}>
            {painel?.tipo === 'novo' && (
              <FormularioDoPacote
                pacote={null}
                base={null}
                aoConcluir={(c) => {
                  setPainel({ tipo: 'pacote', codigo: c });
                }}
                aoFechar={() => {
                  setPainel(null);
                }}
              />
            )}
            {painel?.tipo === 'versao' && atual !== null && (
              <FormularioDoPacote
                pacote={atual}
                base={atual.versoes[0] ?? null}
                aoConcluir={(c) => {
                  setPainel({ tipo: 'pacote', codigo: c });
                }}
                aoFechar={() => {
                  setPainel({ tipo: 'pacote', codigo: atual.codigo });
                }}
              />
            )}
            {painel?.tipo !== 'novo' && painel?.tipo !== 'versao' && atual !== null && (
              <Detalhe
                key={atual.codigo}
                p={atual}
                gerencia={gerencia}
                aoNovaVersao={() => {
                  setPainel({ tipo: 'versao', codigo: atual.codigo });
                }}
              />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
