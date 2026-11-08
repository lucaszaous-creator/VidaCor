import { useQuery } from '@tanstack/react-query';
import type { MovimentoFeito } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Escolha, formatarData } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

/**
 * Registro do contraste aplicado (Estoque.dc.html: "baixa automática pelo exame; sai primeiro o lote
 * que vence antes"). A tela escolhe só o item e a quantidade: o lote é escolhido e baixado pelo
 * servidor, que confere a triagem, a lotação na sala e o saldo (G1, G2). O lote fica ligado ao paciente.
 */
export function RegistroDeContraste({ atendimentoId, paciente }: { atendimentoId: string; paciente: string }) {
  const { api } = useDependencias();
  const itens = useQuery({ queryKey: ['estoque', 'itens'], queryFn: () => api.estoque.itens() });
  const contrastes = (itens.data ?? []).filter((i) => i.contraste);
  const [item, setItem] = useState('');
  const [quantidade, setQuantidade] = useState('1');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((_: undefined, chave: string) =>
    api.estoque.registrarContraste(atendimentoId, { item_id: item, quantidade: Number(quantidade) }, { idempotencyKey: chave }),
  );
  const qtdOk = /^(10|[1-9])$/.test(quantidade.trim());
  const r: MovimentoFeito | null = op.resultado;

  if (r !== null) {
    return (
      <Aviso nivel="feito" titulo={`Contraste registrado para ${paciente}.`}>
        Lote <span className="num">{r.numero_lote}</span>, validade {formatarData(r.validade)}. Restam {r.saldo_do_lote} no lote. O lote fica no histórico do
        paciente para rastreio.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Contraste aplicado"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (item !== '' && qtdOk) op.enviar(undefined);
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Contraste aplicado
      </h2>
      <div className="grid g2" style={{ gap: 10 }}>
        <Escolha
          rotulo="Contraste"
          valor={item}
          aoMudar={setItem}
          vazio="Escolha o contraste"
          opcoes={contrastes.map((c) => ({ valor: c.id, rotulo: `${c.nome} (${String(c.saldo_valido)} em estoque)` }))}
          erro={tentou && item === '' ? 'Escolha o contraste aplicado.' : null}
        />
        <Campo rotulo="Quantidade" value={quantidade} onChange={setQuantidade} inputMode="numeric" erro={tentou && !qtdOk ? 'De 1 a 10.' : null} />
      </div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar o contraste" aoTentarDeNovo={() => { op.enviar(undefined); }} />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Registrar contraste
        </Botao>
      </div>
    </form>
  );
}
