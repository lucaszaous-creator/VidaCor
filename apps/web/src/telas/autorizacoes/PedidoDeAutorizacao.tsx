import { useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { AutorizacaoDetalhe, CanalDaAutorizacao, SituacaoDaAutorizacao } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, Campo, type CorDoChip } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

export const chavesDasAutorizacoes = {
  tudo: ['autorizacoes'] as const,
  lista: ['autorizacoes', 'lista'] as const,
  uma: (id: string) => ['autorizacoes', id] as const,
};

/** Textos e cores das situações vindas do servidor. */
export const SITUACAO_DA_AUTORIZACAO: Record<SituacaoDaAutorizacao, { texto: string; cor: CorDoChip }> = {
  aguardando_envio: { texto: 'Aguardando envio', cor: 'cinza' },
  aguardando_portal: { texto: 'Pedir no portal', cor: 'ambar' },
  enviada: { texto: 'Enviada', cor: 'azul' },
  em_analise: { texto: 'Em análise', cor: 'azul' },
  pendente_documento: { texto: 'Pendente de documento', cor: 'ambar' },
  autorizada: { texto: 'Autorizada', cor: 'verde' },
  parcial: { texto: 'Autorizada em parte', cor: 'ambar' },
  negada: { texto: 'Negada', cor: 'rosa' },
  cancelada: { texto: 'Cancelada', cor: 'cinza' },
  recusada: { texto: 'Recusada pela operadora', cor: 'rosa' },
};

export const CANAL: Record<CanalDaAutorizacao, string> = { webservice: 'Webservice TISS', portal: 'Portal da operadora' };

const DUAS = 'repeat(2, minmax(0, 1fr))';

/**
 * Pedido de autorização a partir da reserva provisória (Agendamento-Guia.dc.html, Autorizacao.dc.html).
 * Os procedimentos são os do agendamento e o canal vem do contrato da operadora: a tela não escolhe.
 */
export function PedidoDeAutorizacao({ agendamentoId }: { agendamentoId: string }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [operadora, setOperadora] = useState('');
  const [carteirinha, setCarteirinha] = useState('');
  const [nome, setNome] = useState('');
  const [conselho, setConselho] = useState('06');
  const [numero, setNumero] = useState('');
  const [uf, setUf] = useState('');
  const [cbos, setCbos] = useState('');
  const [indicacao, setIndicacao] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (_: undefined, chave: string) =>
      api.autorizacoes.pedir(
        {
          agendamento_id: agendamentoId,
          operadora: operadora.trim(),
          carteirinha: carteirinha.trim(),
          solicitante: { nome: nome.trim(), conselho: conselho.trim(), numero: numero.trim(), uf: uf.trim(), cbos: cbos.trim() },
          indicacao_clinica: indicacao.trim() === '' ? null : indicacao.trim(),
        },
        { idempotencyKey: chave },
      ),
    { aoConcluir: () => cliente.invalidateQueries({ queryKey: chavesDasAutorizacoes.tudo }) },
  );
  const falta = (v: string) => (tentou && v.trim() === '' ? 'Preencha este campo.' : null);
  if (op.resultado !== null) return <PedidoFeito d={op.resultado} />;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Pedir autorização"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if ([operadora, carteirinha, nome, conselho, numero, uf, cbos].every((v) => v.trim() !== '')) op.enviar(undefined);
      }}
    >
      <h3 className="t15 b" style={{ margin: 0 }}>
        Pedir autorização à operadora
      </h3>
      <p className="t13 mute" style={{ margin: 0 }}>
        O pedido sai com os procedimentos deste agendamento. Quando a operadora responder, a guia é preenchida sem digitação.
      </p>
      <div className="grid" style={{ gridTemplateColumns: DUAS, gap: 10 }}>
        <Campo rotulo="Operadora do pedido" value={operadora} onChange={setOperadora} erro={falta(operadora)} />
        <Campo rotulo="Carteirinha do pedido" value={carteirinha} onChange={setCarteirinha} erro={falta(carteirinha)} />
        <Campo rotulo="Médico solicitante" value={nome} onChange={setNome} erro={falta(nome)} />
        <Campo rotulo="Número no conselho" value={numero} onChange={setNumero} erro={falta(numero)} />
        <Campo rotulo="Conselho (código TISS)" value={conselho} onChange={setConselho} ajuda="06 = CRM" erro={falta(conselho)} />
        <Campo rotulo="UF do conselho (código IBGE)" value={uf} onChange={setUf} ajuda="33 = RJ" erro={falta(uf)} />
        <Campo rotulo="CBO do solicitante" value={cbos} onChange={setCbos} ajuda="6 dígitos" erro={falta(cbos)} />
      </div>
      <AreaDeTexto rotulo="Indicação clínica (do pedido médico)" value={indicacao} onChange={setIndicacao} linhas={2} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="pedir a autorização" />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Pedir autorização
        </Botao>
      </div>
    </form>
  );
}

function PedidoFeito({ d }: { d: AutorizacaoDetalhe }) {
  const r = d.resumo;
  const texto =
    r.canal === 'webservice'
      ? 'Enviado pelo webservice TISS. A guia é preenchida sozinha quando a operadora responder; negativa ou autorização parcial aparece em Autorizações.'
      : 'Esta operadora não recebe pedido pelo webservice: faça o pedido no portal dela e registre o protocolo e a resposta em Autorizações.';
  return (
    <Aviso nivel={r.canal === 'webservice' ? 'feito' : 'atencao'} titulo={`Pedido de autorização ${r.codigo} criado.`}>
      {texto}{' '}
      <Link to="/autorizacoes" search={{ id: r.id }}>
        Abrir o pedido em Autorizações
      </Link>
    </Aviso>
  );
}
