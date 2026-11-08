import { useQuery } from '@tanstack/react-query';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import { Abas, formatarDinheiro, formatarHora, formatarPontosBase, Vazio } from '@vidacor/design-system';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { useDependencias } from '../../contexto';
import { useRetomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { DecisaoDeDesconto } from './DecisaoDeDesconto';
import { DecisaoDeEstorno } from './DecisaoDeEstorno';

const rota = getRouteApi('/app/caixa/pendencias');
export const chavesDasPendencias = { descontos: ['pendencias', 'descontos'] as const, estornos: ['pendencias', 'estornos'] as const };

/**
 * Pendências do caixa para a gerência e o financeiro (mockup Caixa): pedidos de desconto acima do
 * limite e estornos pedidos. A lista vem do servidor; decidir pede reautenticação e mostra o nome e o
 * nascimento do paciente. Quem pediu não decide (o servidor recusa e a tela mostra o motivo).
 */
export function TelaPendencias() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const navegar = useNavigate();
  const retomada = useRetomada();
  const aba = busca.aba ?? 'descontos';
  const descontos = useQuery({ queryKey: chavesDasPendencias.descontos, queryFn: () => api.pedidosDeDesconto.listar('pedido'), refetchInterval: 15000 });
  const estornos = useQuery({ queryKey: chavesDasPendencias.estornos, queryFn: () => api.estornos.listar('pedido'), refetchInterval: 15000 });
  const abrir = (proxima: { aba: 'descontos' | 'estornos'; id?: string }) => {
    void navegar({ to: '/caixa/pendencias', search: proxima, replace: true });
  };
  const desconto = descontos.data?.find((d) => d.id === busca.id) ?? null;
  const estorno = estornos.data?.find((e) => e.id === busca.id) ?? null;

  const listaDeDescontos = (
    <div>
      <EstadoDaConsulta
        oQue="os pedidos de desconto"
        carregando={descontos.isPending}
        erro={descontos.error}
        aoTentarDeNovo={() => void descontos.refetch()}
        semPermissao={{ titulo: 'Você não decide descontos', detalhe: 'Autorizar desconto acima do limite é da gerência.' }}
      />
      {descontos.data?.length === 0 && <Vazio titulo="Nenhum desconto aguardando." detalhe="Pedidos novos do caixa aparecem aqui sozinhos." />}
      {descontos.data?.map((d) => (
        <LinhaDoPaciente
          key={d.id}
          nome={d.paciente.nome}
          nascimento={d.paciente.nascimento}
          inicio={formatarHora(d.criado_em, FUSO_DA_CLINICA)}
          detalhe={`${formatarPontosBase(d.pontos_base)} de desconto, ${d.motivo}. Pedido por ${d.solicitante.nome}`}
          selecionada={d.id === busca.id}
          aoAbrir={() => {
            abrir({ aba: 'descontos', id: d.id });
          }}
        />
      ))}
    </div>
  );
  const listaDeEstornos = (
    <div>
      <EstadoDaConsulta
        oQue="os pedidos de estorno"
        carregando={estornos.isPending}
        erro={estornos.error}
        aoTentarDeNovo={() => void estornos.refetch()}
        semPermissao={{ titulo: 'Você não decide estornos', detalhe: 'Aprovar estorno é da gerência e do financeiro.' }}
      />
      {estornos.data?.length === 0 && <Vazio titulo="Nenhum estorno aguardando." detalhe="Pedidos novos do caixa aparecem aqui sozinhos." />}
      {estornos.data?.map((e) => (
        <LinhaDoPaciente
          key={e.id}
          nome={e.paciente.nome}
          nascimento={e.paciente.nascimento}
          inicio={formatarHora(e.pedido_em, FUSO_DA_CLINICA)}
          detalhe={`${formatarDinheiro(e.total_centavos)}, ${e.recebimento_codigo}. Pedido por ${e.pedido_por.nome}`}
          selecionada={e.id === busca.id}
          aoAbrir={() => {
            abrir({ aba: 'estornos', id: e.id });
          }}
        />
      ))}
    </div>
  );

  return (
    <>
      <CabecalhoDaTela titulo="Pendências do caixa" complemento={<span className="t14 mute">Descontos acima do limite e estornos pedidos</span>} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Pendências">
          <div className="scroll">
            <Abas
              rotulo="Tipo de pendência"
              selecionada={aba}
              aoSelecionar={(id) => {
                abrir({ aba: id === 'estornos' ? 'estornos' : 'descontos' });
              }}
              abas={[
                { id: 'descontos', rotulo: `Descontos (${String(descontos.data?.length ?? 0)})`, conteudo: listaDeDescontos },
                { id: 'estornos', rotulo: `Estornos (${String(estornos.data?.length ?? 0)})`, conteudo: listaDeEstornos },
              ]}
            />
          </div>
        </section>
        <section className="card pane" aria-label="Decisão">
          <div className="scroll col" style={{ gap: 14 }}>
            {aba === 'descontos' && desconto !== null && <DecisaoDeDesconto key={desconto.id} pedido={desconto} retomada={retomada} />}
            {aba === 'estornos' && estorno !== null && <DecisaoDeEstorno key={estorno.id} estorno={estorno} retomada={retomada} />}
            {((aba === 'descontos' && desconto === null) || (aba === 'estornos' && estorno === null)) && (
              <Vazio titulo="Escolha uma pendência na lista." detalhe="O detalhe mostra o paciente, quem pediu e o motivo antes da decisão." />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
