import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { MudancaNoProcedimento, MudancaRegistradaNoCatalogo, ProcedimentoDoCatalogo } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Chip, formatarDataHora, formatarDinheiro, ListaDeDados, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CHAVE_CATALOGO, dataCurta } from './comum';

const TIPO: Record<MudancaRegistradaNoCatalogo['tipo'], string> = {
  incluir: 'Incluído',
  alterar: 'Alterado',
  desativar: 'Desativado',
  reativar: 'Reativado',
};

function Historico({ tuss }: { tuss: string }) {
  const { api } = useDependencias();
  const h = useQuery({
    queryKey: [...CHAVE_CATALOGO, tuss, 'historico'],
    queryFn: () => api.catalogo.historico(tuss),
  });
  return (
    <div className="col" style={{ gap: 8 }}>
      <h3 className="t15 b" style={{ margin: 0 }}>
        Histórico
      </h3>
      <EstadoDaConsulta oQue="o histórico" carregando={h.isPending} erro={h.error} aoTentarDeNovo={() => void h.refetch()} />
      {h.data?.length === 0 && <Vazio titulo="Sem mudanças registradas pelo sistema." detalhe="O cadastro inicial veio da carga do catálogo." />}
      <ul className="col" style={{ gap: 8, listStyle: 'none', margin: 0, padding: 0 }}>
        {h.data?.map((m) => (
          <li key={m.codigo} style={{ borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
            <div className="row" style={{ gap: 8 }}>
              <b style={{ flex: 1, minWidth: 0 }}>
                {TIPO[m.tipo]}, versão {m.versao_do_procedimento}
              </b>
              <span className="t13 mute">{formatarDataHora(m.em, FUSO_DA_CLINICA)}</span>
            </div>
            <div className="t13 mute">
              {m.quem}: {m.motivo}.
              {m.antes !== null &&
                m.antes !== undefined &&
                ` Antes: ${String(m.antes.duracao_min)} min, ${m.antes.modalidade ?? 'sem modalidade'}, ${m.antes.ativo ? 'ativo' : 'inativo'}.`}
              {` Depois: ${String(m.depois.duracao_min)} min, ${m.depois.modalidade ?? 'sem modalidade'}, ${m.depois.ativo ? 'ativo' : 'inativo'}.`}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

function Mudanca({ p }: { p: ProcedimentoDoCatalogo }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [duracao, setDuracao] = useState(String(p.duracao_min));
  const [modalidade, setModalidade] = useState(p.modalidade ?? '');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((m: MudancaNoProcedimento, chave: string) => api.catalogo.mudar(p.tuss, m, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: CHAVE_CATALOGO }),
  });
  const dur = Number(duracao);
  const durOk = /^\d{1,3}$/.test(duracao.trim()) && dur >= 5 && dur <= 240 && dur % 5 === 0;
  const modOk = /^[A-Za-z]{2,4}$/.test(modalidade.trim());
  const motivoOk = motivo.trim().length >= 5;
  const enviar = (ativo?: boolean) => {
    setTentou(true);
    if (!motivoOk) return;
    if (ativo === undefined) {
      if (durOk && modOk)
        op.enviar({
          versao: p.versao,
          duracao_min: dur,
          modalidade: modalidade.trim().toUpperCase(),
          motivo: motivo.trim(),
        });
    } else {
      op.enviar({ versao: p.versao, ativo, motivo: motivo.trim() });
    }
  };
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Mudar cadastro do exame"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        enviar();
      }}
    >
      <h3 className="t15 b" style={{ margin: 0 }}>
        Mudar cadastro
      </h3>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <Campo rotulo="Duração na agenda (min)" value={duracao} onChange={setDuracao} inputMode="numeric" erro={tentou && !durOk ? 'De 5 a 240, de 5 em 5.' : null} />
        <Campo rotulo="Modalidade" value={modalidade} onChange={setModalidade} ajuda="Ex.: US, CT, DX" erro={tentou && !modOk ? 'Use 2 a 4 letras.' : null} />
      </div>
      <Campo rotulo="Motivo da mudança" value={motivo} onChange={setMotivo} erro={tentou && !motivoOk ? 'Diga o motivo (5 letras ou mais).' : null} />
      <div aria-live="polite">
        {op.resultado !== null && op.erro === null && (
          <Aviso nivel="feito" titulo={`${op.resultado.nome}: versão ${String(op.resultado.versao)} registrada.`}>
            Vale para os próximos agendamentos. Registrado no histórico e no lastro.
          </Aviso>
        )}
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="mudar o exame" />}
      </div>
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Salvar mudança
        </Botao>
        <Botao
          onPress={() => {
            enviar(!p.ativo);
          }}
        >
          {p.ativo ? 'Desativar exame' : 'Reativar exame'}
        </Botao>
      </div>
    </form>
  );
}

/** Detalhe do exame: cadastro vigente, preço desta unidade, mudança (gerência) e histórico. */
export function DetalheDoExame({ p, gerencia, proximaVigencia }: { p: ProcedimentoDoCatalogo; gerencia: boolean; proximaVigencia: string | null }) {
  return (
    <>
      <div className="row" style={{ gap: 10 }}>
        <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{p.nome}</h2>
        <Chip cor={p.ativo ? 'verde' : 'cinza'} texto={p.ativo ? 'Ativo' : 'Inativo'} />
      </div>
      {!p.ativo && (
        <Aviso nivel="informativo" titulo="Exame desativado.">
          Não aparece para agendar nem entra em tabela de preços nova. O histórico continua guardado.
        </Aviso>
      )}
      <ListaDeDados
        larguraDoRotulo={170}
        itens={[
          {
            rotulo: 'Código TUSS',
            valor: <span className="num">{p.tuss}</span>,
          },
          { rotulo: 'Modalidade', valor: p.modalidade ?? 'Sem modalidade' },
          {
            rotulo: 'Duração na agenda',
            valor: `${String(p.duracao_min)} min`,
          },
          {
            rotulo: 'Contraste',
            valor: p.requer_contraste ? 'Pede contraste (triagem obrigatória)' : 'Sem contraste',
          },
          {
            rotulo: 'Preço particular',
            valor: typeof p.preco_centavos === 'number' ? <span className="num">{formatarDinheiro(p.preco_centavos)}</span> : 'Sem preço na tabela vigente',
          },
          ...(typeof p.proximo_preco_centavos === 'number' && proximaVigencia !== null
            ? [
                {
                  rotulo: `A partir de ${dataCurta(proximaVigencia)}`,
                  valor: <span className="num">{formatarDinheiro(p.proximo_preco_centavos)}</span>,
                },
              ]
            : []),
          {
            rotulo: 'Versão do cadastro',
            valor: <span className="num">{p.versao}</span>,
          },
        ]}
      />
      {gerencia && <Mudanca p={p} />}
      <Historico tuss={p.tuss} />
    </>
  );
}
