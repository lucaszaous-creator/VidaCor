import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { RepasseDoMedico } from '@vidacor/api-client';
import { Aviso, Botao, Escolha, formatarDinheiro, formatarPontosBase, Tabela } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

function competencias(): { valor: string; rotulo: string }[] {
  const hoje = new Date();
  return [1, 2, 3].map((k) => {
    const d = new Date(hoje.getFullYear(), hoje.getMonth() - k, 1);
    const valor = `${String(d.getFullYear())}-${String(d.getMonth() + 1).padStart(2, '0')}`;
    return { valor, rotulo: valor.split('-').reverse().join('/') };
  });
}

/** Repasse médico da competência fechada, pela regra versionada; gerar não duplica. */
export function Repasse() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const opcoes = competencias();
  const [comp, setComp] = useState(opcoes[0]?.valor ?? '');
  const q = useQuery({ queryKey: ['financeiro', 'repasse', comp], queryFn: () => api.financeiro.repasse(comp) });
  const gerar = useOperacao((_: undefined, chave: string) => api.financeiro.gerarRepasse(comp, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['financeiro'] }),
  });
  return (
    <div className="col" style={{ gap: 10 }}>
      <div className="row" style={{ gap: 8, alignItems: 'flex-end' }}>
        <div style={{ width: 200 }}>
          <Escolha rotulo="Competência" valor={comp} aoMudar={(c) => { setComp(c); gerar.reiniciar(); }} opcoes={opcoes} />
        </div>
        <Botao variante="principal" ocupado={gerar.enviando} onPress={() => { gerar.enviar(undefined); }}>
          Gerar títulos do repasse
        </Botao>
      </div>
      <EstadoDaConsulta oQue="o repasse" carregando={q.isPending} erro={q.error} aoTentarDeNovo={() => void q.refetch()} />
      {q.data !== undefined && (
        <Tabela<RepasseDoMedico & { id: string }>
          rotulo="Repasse por médico"
          vazio="Nenhum laudo assinado na competência."
          linhas={q.data.map((m) => ({ ...m, id: m.medico_id }))}
          colunas={[
            { id: 'medico', rotulo: 'Médico', cabecalhoDaLinha: true, celula: (m) => m.medico },
            { id: 'laudos', rotulo: 'Laudos', direita: true, semQuebra: true, celula: (m) => m.laudos },
            { id: 'base', rotulo: 'Base', direita: true, semQuebra: true, celula: (m) => formatarDinheiro(m.base_centavos) },
            {
              id: 'regra',
              rotulo: 'Regra',
              semQuebra: true,
              celula: (m) => (typeof m.percentual_pb === 'number' ? `${formatarPontosBase(m.percentual_pb)}, v${String(m.regra_versao)}` : <b style={{ color: 'var(--rose)' }}>Sem regra</b>),
            },
            { id: 'valor', rotulo: 'Repasse', direita: true, semQuebra: true, celula: (m) => (typeof m.valor_centavos === 'number' ? formatarDinheiro(m.valor_centavos) : 'Não calculado') },
          ]}
        />
      )}
      {gerar.erro !== null && <ErroDoAto erro={gerar.erro} ato="gerar o repasse" />}
      {gerar.resultado !== null && (
        <Aviso nivel="feito" titulo={`${String(gerar.resultado.titulos_criados)} título(s) criado(s), ${String(gerar.resultado.ja_existiam)} já existia(m).`}>
          {gerar.resultado.sem_regra.length > 0 ? `Sem regra de repasse, nada foi gerado para: ${gerar.resultado.sem_regra.join(', ')}.` : 'Os títulos aguardam aprovação na aba A pagar.'}
        </Aviso>
      )}
    </div>
  );
}
