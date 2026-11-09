import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { ConvenioDetalhe, NovasRegrasDeAutorizacao, RegraDeAutorizacaoPedida } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Chip, Escolha, formatarDataHora, Tabela, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CHAVE_CONVENIOS } from './comum';

interface Linha {
  tuss: string;
  nome: string;
  senha: string;
  validade: string;
  documentos: string;
}

/** Versão nova das regras de autorização (faturamento): a lista inteira, com motivo. */
function NovasRegras({ c, aoFechar }: { c: ConvenioDetalhe; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const atuais = c.regras_autorizacao?.itens ?? [];
  const precos = useQuery({
    queryKey: [...CHAVE_CONVENIOS, c.id, 'precos', ''],
    queryFn: () => api.convenios.precos(c.id),
    enabled: atuais.length === 0 && c.tabelas.length > 0,
  });
  const base: Linha[] =
    atuais.length > 0
      ? atuais.map((r) => ({ tuss: r.tuss, nome: r.nome, senha: r.exige_senha ? 'sim' : 'nao', validade: typeof r.validade_dias === 'number' ? String(r.validade_dias) : '', documentos: r.documentos }))
      : (precos.data?.itens ?? []).map((p) => ({ tuss: p.tuss, nome: p.descricao, senha: 'nao', validade: '', documentos: 'Pedido médico e carteirinha' }));
  const [linhas, setLinhas] = useState<Linha[] | null>(null);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const atual = linhas ?? base;
  const op = useOperacao((n: NovasRegrasDeAutorizacao, chave: string) => api.convenios.publicarRegras(c.id, n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: CHAVE_CONVENIOS }),
  });
  const mudar = (tuss: string, campo: keyof Linha, valor: string) => {
    setLinhas(atual.map((l) => (l.tuss === tuss ? { ...l, [campo]: valor } : l)));
  };
  const invalidas = atual.filter((l) => (l.senha === 'sim' && !/^\d{1,3}$/.test(l.validade.trim())) || l.documentos.trim().length < 3).map((l) => l.tuss);
  if (op.resultado !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={`Regras v${String(op.resultado.versao_regras)} publicadas.`}
        acoes={
          <Botao pequeno onPress={aoFechar}>
            Fechar
          </Botao>
        }
      >
        A versão anterior continua no histórico.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Regras de autorização"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (invalidas.length === 0 && motivo.trim().length >= 5 && atual.length > 0) {
          const itens: RegraDeAutorizacaoPedida[] = atual.map((l) => ({
            tuss: l.tuss,
            exige_senha: l.senha === 'sim',
            validade_dias: l.senha === 'sim' ? Number(l.validade.trim()) : null,
            documentos: l.documentos.trim(),
          }));
          op.enviar({ versao_vista: c.regras_autorizacao?.versao_regras ?? null, motivo: motivo.trim(), itens });
        }
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Regras de autorização, {c.operadora}
      </h2>
      {atual.length === 0 && <Vazio titulo="Sem exames para regrar." detalhe="As regras partem da tabela de preços do convênio." />}
      {atual.map((l) => (
        <fieldset key={l.tuss} className="col" style={{ gap: 6, border: 0, borderBottom: '1px solid var(--line)', padding: '0 0 8px', margin: 0 }}>
          <legend className="t14 b">
            {l.nome} <span className="num t13 mute">{l.tuss}</span>
          </legend>
          <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 0.6fr) minmax(0, 1.4fr)', gap: 8, alignItems: 'start' }}>
            <Escolha
              rotulo={`Senha para ${l.nome}`}
              valor={l.senha}
              aoMudar={(v) => {
                mudar(l.tuss, 'senha', v);
              }}
              opcoes={[
                { valor: 'nao', rotulo: 'Só elegibilidade' },
                { valor: 'sim', rotulo: 'Pede senha' },
              ]}
            />
            <Campo
              rotulo={`Validade em dias para ${l.nome}`}
              value={l.validade}
              onChange={(v) => {
                mudar(l.tuss, 'validade', v);
              }}
              isDisabled={l.senha !== 'sim'}
              inputMode="numeric"
              erro={tentou && invalidas.includes(l.tuss) && l.senha === 'sim' ? 'De 1 a 365.' : null}
            />
            <Campo
              rotulo={`Documentos para ${l.nome}`}
              value={l.documentos}
              onChange={(v) => {
                mudar(l.tuss, 'documentos', v);
              }}
            />
          </div>
        </fieldset>
      ))}
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Diga o motivo (5 letras ou mais).' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="publicar as regras" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Publicar versão nova
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
    </form>
  );
}

/** Aba "Autorização": o que cada exame exige da operadora, por versão. */
export function RegrasDoConvenio({ c, faturista }: { c: ConvenioDetalhe; faturista: boolean }) {
  const [editando, setEditando] = useState(false);
  if (editando) {
    return (
      <NovasRegras
        c={c}
        aoFechar={() => {
          setEditando(false);
        }}
      />
    );
  }
  const r = c.regras_autorizacao;
  return (
    <div className="col" style={{ gap: 12 }}>
      {r === null || r === undefined ? (
        <Vazio titulo="Sem regras de autorização cadastradas." detalhe="Cadastre o que a operadora exige de cada exame: senha, validade e documentos." />
      ) : (
        <>
          <Tabela
            rotulo="Regras de autorização"
            colunas={[
              { id: 'exame', rotulo: 'Exame', cabecalhoDaLinha: true, celula: (x) => <span className="b">{x.nome}</span> },
              { id: 'senha', rotulo: 'Senha', celula: (x) => <Chip cor={x.exige_senha ? 'ambar' : 'verde'} texto={x.exige_senha ? 'Pede senha' : 'Só elegibilidade'} /> },
              { id: 'validade', rotulo: 'Validade', celula: (x) => (typeof x.validade_dias === 'number' ? `${String(x.validade_dias)} dias` : 'Não se aplica') },
              { id: 'docs', rotulo: 'Documentos exigidos', celula: (x) => x.documentos },
            ]}
            linhas={r.itens.map((x) => ({ ...x, id: x.tuss }))}
          />
          <div className="t13 mute">
            Regras v{r.versao_regras}, publicadas por {r.publicada_por} em {formatarDataHora(r.publicada_em, FUSO_DA_CLINICA)}: {r.motivo}.
          </div>
        </>
      )}
      {faturista && (
        <div className="row">
          <Botao
            pequeno
            onPress={() => {
              setEditando(true);
            }}
          >
            Nova versão das regras
          </Botao>
        </div>
      )}
    </div>
  );
}
