import type { CalendarDate } from '@internationalized/date';
import { useQueryClient } from '@tanstack/react-query';
import type { CatalogoDeExames, NovaVersaoDaTabela, NovoProcedimento, VersaoDaTabelaDePreco } from '@vidacor/api-client';
import { Aviso, Botao, Campo, CampoDeData, CaixaDeMarcar, Escolha, formatarDataHora, lerReais } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CHAVE_CATALOGO, dataCurta, reaisParaCampo } from './comum';

const MODALIDADES = [
  { valor: 'US', rotulo: 'Ultrassonografia (US)' },
  { valor: 'DX', rotulo: 'Raio X (DX)' },
  { valor: 'CT', rotulo: 'Tomografia (CT)' },
  { valor: 'MR', rotulo: 'Ressonância (MR)' },
  { valor: 'MG', rotulo: 'Mamografia (MG)' },
];
const DURACOES = [10, 15, 20, 30, 40, 50, 60, 90].map((d) => ({
  valor: String(d),
  rotulo: `${String(d)} min`,
}));

/** Inclui exame no catálogo (gerência). O servidor confere o TUSS único e a duração. */
export function IncluirExame({ aoIncluir, aoFechar }: { aoIncluir: (tuss: string) => void; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [nome, setNome] = useState('');
  const [tuss, setTuss] = useState('');
  const [modalidade, setModalidade] = useState('');
  const [duracao, setDuracao] = useState('');
  const [contraste, setContraste] = useState(false);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoProcedimento, chave: string) => api.catalogo.incluir(n, { idempotencyKey: chave }), {
    aoConcluir: async (p) => {
      await cliente.invalidateQueries({ queryKey: CHAVE_CATALOGO });
      aoIncluir(p.tuss);
    },
  });
  const tussOk = /^\d{8}$/.test(tuss.trim());
  const ok = nome.trim().length >= 3 && tussOk && modalidade !== '' && duracao !== '' && motivo.trim().length >= 5;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Incluir exame"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok)
          op.enviar({
            nome: nome.trim(),
            tuss: tuss.trim(),
            modalidade,
            duracao_min: Number(duracao),
            requer_contraste: contraste,
            motivo: motivo.trim(),
          });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Incluir exame
      </h2>
      <Campo rotulo="Nome do exame" value={nome} onChange={setNome} erro={tentou && nome.trim().length < 3 ? 'Diga o nome do exame.' : null} />
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <Campo rotulo="Código TUSS" value={tuss} onChange={setTuss} inputMode="numeric" erro={tentou && !tussOk ? 'O código TUSS tem 8 algarismos.' : null} />
        <Escolha rotulo="Duração na agenda" valor={duracao} aoMudar={setDuracao} vazio="Escolha" opcoes={DURACOES} erro={tentou && duracao === '' ? 'Escolha a duração.' : null} />
      </div>
      <Escolha
        rotulo="Modalidade"
        valor={modalidade}
        aoMudar={setModalidade}
        vazio="Escolha a modalidade"
        opcoes={MODALIDADES}
        erro={tentou && modalidade === '' ? 'Escolha a modalidade.' : null}
      />
      <CaixaDeMarcar rotulo="Pede contraste" marcado={contraste} aoMudar={setContraste} detalhe="A triagem de segurança passa a ser obrigatória antes do exame." />
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Diga o motivo (5 letras ou mais).' : null} />
      <p className="t13 mute" style={{ margin: 0 }}>
        Sem preço na tabela, o exame não é cobrado no caixa: publique uma versão nova da tabela com o preço dele.
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="incluir o exame" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Incluir exame
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
    </form>
  );
}

const SITUACAO_DA_TABELA: Record<VersaoDaTabelaDePreco['situacao'], string> = {
  vigente: 'vigente',
  futura: 'começa no futuro',
  anterior: 'anterior',
};

/**
 * Versão nova da tabela de preços (gerência). Parte dos preços vigentes; começa numa data futura e não
 * altera a vigente. Exame sem preço fica fora da versão nova.
 */
export function NovaTabelaDePrecos({ catalogo, aoFechar }: { catalogo: CatalogoDeExames; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const ativos = catalogo.procedimentos.filter((p) => p.ativo);
  const [precos, setPrecos] = useState<Record<string, string>>(() =>
    Object.fromEntries(ativos.map((p) => [p.tuss, typeof p.preco_centavos === 'number' ? reaisParaCampo(p.preco_centavos) : ''])),
  );
  const [desde, setDesde] = useState<CalendarDate | null>(null);
  const [descricao, setDescricao] = useState('');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovaVersaoDaTabela, chave: string) => api.catalogo.publicarTabela(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: CHAVE_CATALOGO }),
  });
  const invalidos = ativos.filter((p) => (precos[p.tuss] ?? '').trim() !== '' && lerReais(precos[p.tuss] ?? '') === null).map((p) => p.tuss);
  const itens = ativos.flatMap((p) => {
    const v = lerReais(precos[p.tuss] ?? '');
    return v === null || v <= 0 ? [] : [{ tuss: p.tuss, valor_centavos: v }];
  });
  const ok = desde !== null && descricao.trim().length >= 3 && motivo.trim().length >= 5 && invalidos.length === 0 && itens.length > 0;
  if (op.resultado !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={`Tabela v${String(op.resultado.versao)} publicada; começa em ${dataCurta(op.resultado.vigente_desde)}.`}
        acoes={
          <Botao pequeno onPress={aoFechar}>
            Fechar
          </Botao>
        }
      >
        {op.resultado.itens} preços. Até lá, vale a tabela vigente; orçamentos já emitidos mantêm o valor. Registrado no lastro.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Nova tabela de preços"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok)
          op.enviar({
            vigente_desde: desde.toString(),
            descricao: descricao.trim(),
            motivo: motivo.trim(),
            precos: itens,
          });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Nova tabela de preços
      </h2>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <CampoDeData rotulo="Começa em" valor={desde} aoMudar={setDesde} ajuda="A partir de amanhã" erro={tentou && desde === null ? 'Diga quando começa.' : null} />
        <Campo
          rotulo="Nome da versão"
          value={descricao}
          onChange={setDescricao}
          placeholder="Particular 2027"
          erro={tentou && descricao.trim().length < 3 ? 'Dê um nome à versão.' : null}
        />
      </div>
      <fieldset className="col" style={{ gap: 6, border: 0, padding: 0, margin: 0 }}>
        <legend className="t13 b">Preço particular por exame (em reais; vazio fica fora)</legend>
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)',
            gap: '6px 10px',
            alignItems: 'end',
          }}
        >
          {ativos.map((p) => (
            <Campo
              key={p.tuss}
              rotulo={`${p.nome} (TUSS ${p.tuss})`}
              value={precos[p.tuss] ?? ''}
              onChange={(v) => {
                setPrecos((x) => ({ ...x, [p.tuss]: v }));
              }}
              inputMode="decimal"
              erro={tentou && invalidos.includes(p.tuss) ? 'Valor em reais, ex.: 380,00.' : null}
            />
          ))}
        </div>
        {tentou && itens.length === 0 && <span className="campo-erro">Informe ao menos um preço.</span>}
      </fieldset>
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Diga o motivo (ex.: reajuste anual).' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="publicar a tabela" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Publicar versão
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
      <h3 className="t15 b" style={{ margin: '8px 0 0' }}>
        Versões publicadas
      </h3>
      <ul className="col" style={{ gap: 6, listStyle: 'none', margin: 0, padding: 0 }}>
        {catalogo.tabelas.map((t) => (
          <li key={t.id} className="t13" style={{ borderBottom: '1px solid var(--line)', paddingBottom: 6 }}>
            <b>
              v{t.versao} {t.descricao}
            </b>
            , desde {dataCurta(t.vigente_desde)} ({SITUACAO_DA_TABELA[t.situacao]}), {t.itens} preços.
            <span className="mute">
              {' '}
              Publicada {typeof t.publicada_por === 'string' ? `por ${t.publicada_por} ` : ''}
              em {formatarDataHora(t.publicada_em, FUSO_DA_CLINICA)}
              {typeof t.motivo === 'string' ? `: ${t.motivo}` : ''}.
            </span>
          </li>
        ))}
      </ul>
    </form>
  );
}
