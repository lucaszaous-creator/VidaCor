import type { CalendarDate } from '@internationalized/date';
import { useQueryClient } from '@tanstack/react-query';
import type { NovoTitulo } from '@vidacor/api-client';
import { Aviso, Botao, Campo, CampoDeData, Escolha, formatarDinheiro } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const CATEGORIAS = ['fornecedor', 'folha', 'repasse', 'imposto', 'operadora', 'particular', 'outros'].map((c) => ({ valor: c, rotulo: c.charAt(0).toUpperCase() + c.slice(1) }));

function centavos(texto: string): number | null {
  const limpo = texto.replace(/\./g, '').replace(',', '.').trim();
  if (!/^\d+(\.\d{1,2})?$/.test(limpo)) return null;
  return Math.round(Number(limpo) * 100);
}

/** Novo lançamento: o servidor decide quantas aprovações o título precisa. */
export function NovoLancamento() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [tipo, setTipo] = useState('pagar');
  const [contraparte, setContraparte] = useState('');
  const [descricao, setDescricao] = useState('');
  const [categoria, setCategoria] = useState('fornecedor');
  const [valor, setValor] = useState('');
  const [vencimento, setVencimento] = useState<CalendarDate | null>(null);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoTitulo, chave: string) => api.financeiro.lancar(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['financeiro'] }),
  });
  const v = centavos(valor);
  const ok = contraparte.trim().length >= 2 && descricao.trim().length >= 3 && v !== null && v > 0 && vencimento !== null;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok) {
          op.enviar({ tipo: tipo as 'pagar' | 'receber', contraparte: contraparte.trim(), descricao: descricao.trim(), categoria, valor_centavos: v, vencimento: vencimento.toString() });
        }
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Novo lançamento
      </h2>
      <Escolha rotulo="Tipo" valor={tipo} aoMudar={setTipo} opcoes={[{ valor: 'pagar', rotulo: 'A pagar' }, { valor: 'receber', rotulo: 'A receber' }]} />
      <Campo rotulo={tipo === 'pagar' ? 'Para quem' : 'De quem'} value={contraparte} onChange={setContraparte} erro={tentou && contraparte.trim().length < 2 ? 'Diga a contraparte.' : null} />
      <Campo rotulo="Descrição" value={descricao} onChange={setDescricao} erro={tentou && descricao.trim().length < 3 ? 'Descreva o lançamento.' : null} />
      <Escolha rotulo="Categoria" valor={categoria} aoMudar={setCategoria} opcoes={CATEGORIAS} />
      <Campo rotulo="Valor (R$)" value={valor} onChange={setValor} inputMode="decimal" erro={tentou && (v === null || v <= 0) ? 'Digite o valor, ex.: 1.250,00' : null} />
      <CampoDeData rotulo="Vencimento" valor={vencimento} aoMudar={setVencimento} erro={tentou && vencimento === null ? 'Digite o vencimento.' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="lançar o título" />}
      {op.resultado !== null && (
        <Aviso nivel="feito" titulo={`${op.resultado.codigo} lançado: ${formatarDinheiro(op.resultado.valor_centavos)}.`}>
          {op.resultado.aprovacoes_exigidas === 0 ? 'Não precisa de aprovação.' : `Precisa de ${String(op.resultado.aprovacoes_exigidas)} aprovação(ões) de outra pessoa antes do pagamento.`}
        </Aviso>
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Lançar
        </Botao>
      </div>
    </form>
  );
}
