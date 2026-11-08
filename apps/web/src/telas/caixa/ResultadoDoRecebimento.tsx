import { useQuery } from '@tanstack/react-query';
import type { Estorno, Recebimento } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, BotaoDeIcone, Chip, DialogoDeFormulario, formatarDinheiro, type CorDoChip } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const SITUACAO: Record<string, { texto: string; cor: CorDoChip }> = {
  aguardando_pagamento: { texto: 'Aguardando pagamento', cor: 'ambar' },
  pago: { texto: 'Pago', cor: 'verde' },
  estornado: { texto: 'Estornado', cor: 'cinza' },
};

function CopiaECola({ codigo }: { codigo: string }) {
  const [copiado, setCopiado] = useState<'sim' | 'falhou' | null>(null);
  return (
    <div className="col" style={{ gap: 6 }}>
      <span className="t13 b mute">PIX copia e cola</span>
      <div className="row" style={{ gap: 8 }}>
        <code className="t13 num" style={{ flex: 1, minWidth: 0, overflowWrap: 'anywhere', background: 'var(--gray-bg)', padding: '8px 10px', borderRadius: 10 }}>
          {codigo}
        </code>
        <BotaoDeIcone
          icone="copiar"
          rotulo="Copiar o código PIX"
          onPress={() => {
            navigator.clipboard.writeText(codigo).then(
              () => {
                setCopiado('sim');
              },
              () => {
                setCopiado('falhou');
              },
            );
          }}
        />
      </div>
      <span className="t13" role="status">
        {copiado === 'sim' ? 'Código copiado.' : copiado === 'falhou' ? 'Não foi possível copiar: selecione o código e copie à mão.' : ''}
      </span>
    </div>
  );
}

function PedidoDeEstorno({ recebimento }: { recebimento: Recebimento }) {
  const { api } = useDependencias();
  const [aberto, setAberto] = useState(false);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((m: string, chave: string) => api.recebimentos.pedirEstorno(recebimento.id, { motivo: m }, { idempotencyKey: chave }), {
    aoConcluir: () => {
      setAberto(false);
    },
  });
  const estorno: Estorno | null = op.resultado;
  if (estorno !== null) {
    return (
      <Aviso nivel="feito" titulo="Estorno pedido.">
        Pedido <b className="num">{estorno.id}</b>. Quem aprova é a gerência ou o financeiro; quem pediu não aprova o próprio pedido. Envie o
        endereço <span className="num">/caixa/estornos?estorno={estorno.id}</span> a quem vai aprovar.
      </Aviso>
    );
  }
  return (
    <>
      <div>
        <Botao
          pequeno
          onPress={() => {
            setAberto(true);
          }}
        >
          Pedir estorno
        </Botao>
      </div>
      <DialogoDeFormulario
        aberto={aberto}
        titulo={`Pedir o estorno do recebimento ${recebimento.codigo}?`}
        rotuloConfirmar="Pedir estorno"
        ocupado={op.enviando}
        aoFechar={() => {
          setAberto(false);
        }}
        aoConfirmar={() => {
          setTentou(true);
          if (motivo.trim() !== '') op.enviar(motivo.trim());
        }}
      >
        <p className="t14" style={{ margin: 0 }}>
          Total {formatarDinheiro(recebimento.total_centavos)}. O estorno só acontece depois da aprovação de outra pessoa.
        </p>
        <AreaDeTexto rotulo="Motivo do estorno" value={motivo} onChange={setMotivo} linhas={2} erro={tentou && motivo.trim() === '' ? 'Escreva o motivo.' : null} />
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="pedir o estorno" aoTentarDeNovo={() => { op.enviar(motivo.trim()); }} />}
      </DialogoDeFormulario>
    </>
  );
}

/**
 * Recebimento como o servidor calculou: itens, subtotal, desconto, total e versão da tabela
 * (FC-CXA-01). O PIX copia e cola vem `null` até o PSP devolver a cobrança (docs/30 § 7): a tela
 * consulta o recebimento até ele aparecer, e depois segue a situação até o pagamento.
 */
export function ResultadoDoRecebimento({ r: inicial }: { r: Recebimento }) {
  const { api, intervaloDoRecebimentoMs = 2000 } = useDependencias();
  const consulta = useQuery({
    queryKey: ['recebimento', inicial.id],
    queryFn: () => api.recebimentos.obter(inicial.id),
    initialData: inicial,
    refetchInterval: (q) => {
      const atual = q.state.data;
      if (atual === undefined) return false;
      const semPix = atual.forma === 'pix' && (atual.pix === null || atual.pix === undefined);
      return semPix || atual.situacao === 'aguardando_pagamento' ? intervaloDoRecebimentoMs : false;
    },
  });
  const r = consulta.data;
  const pix = r.pix ?? null;
  const s = SITUACAO[r.situacao] ?? { texto: 'Situação não reconhecida', cor: 'cinza' as const };
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h3 className="t15 b">
          Recebimento <span className="num">{r.codigo}</span>
        </h3>
        <Chip cor={s.cor} texto={s.texto} />
      </div>
      <table className="tb" aria-label="Itens calculados pelo servidor">
        <thead>
          <tr>
            <th>Procedimento</th>
            <th className="r nw">Valor</th>
          </tr>
        </thead>
        <tbody>
          {r.itens.map((i) => (
            <tr key={i.tuss}>
              <td>
                {i.descricao} <span className="mute num">TUSS {i.tuss}</span>
              </td>
              <td className="r nw num">{formatarDinheiro(i.valor_centavos)}</td>
            </tr>
          ))}
          <tr>
            <td>Subtotal</td>
            <td className="r nw num">{formatarDinheiro(r.subtotal_centavos)}</td>
          </tr>
          <tr>
            <td>Desconto</td>
            <td className="r nw num">{formatarDinheiro(r.desconto_centavos)}</td>
          </tr>
          <tr>
            <td>
              <b>Total</b>
            </td>
            <td className="r nw num">
              <b>{formatarDinheiro(r.total_centavos)}</b>
            </td>
          </tr>
        </tbody>
      </table>
      <span className="t13 mute">
        Valores calculados pelo servidor pela tabela versão <span className="num">{r.tabela_versao}</span>.
      </span>
      {r.forma === 'pix' && pix === null && (
        <Aviso nivel="informativo" titulo="Gerando a cobrança PIX.">
          O recebimento foi registrado; o código copia e cola aparece aqui assim que o banco devolver a cobrança.
        </Aviso>
      )}
      {pix !== null && (
        <>
          <CopiaECola codigo={pix.copia_e_cola} />
          <span className="t13 mute num">txid {pix.txid}. A situação acima muda para pago quando o banco confirmar.</span>
        </>
      )}
      {r.situacao !== 'aguardando_pagamento' && <NotaDoRecebimento recebimentoId={r.id} />}
      <PedidoDeEstorno recebimento={r} />
    </div>
  );
}

/** NFS-e do recebimento: aparece quando a prefeitura devolver (a emissão é pelo outbox). */
function NotaDoRecebimento({ recebimentoId }: { recebimentoId: string }) {
  const { api } = useDependencias();
  const nota = useQuery({
    queryKey: ['nota', recebimentoId],
    queryFn: () => api.recebimentos.nota(recebimentoId),
    retry: false,
    refetchInterval: (q) => (q.state.data === undefined ? 5000 : false),
  });
  if (nota.data === undefined) {
    return <span className="t13 mute">NFS-e: emissão em andamento; o número aparece aqui.</span>;
  }
  return (
    <span className="t13" role="status">
      NFS-e <span className="num">{nota.data.numero}</span>, código {nota.data.codigo_verificacao}
      {nota.data.situacao === 'cancelada' ? ', cancelada pelo estorno' : ''}.
    </span>
  );
}
