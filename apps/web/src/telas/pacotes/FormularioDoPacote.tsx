import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { NovoPacoteDeCheckup, PacoteDeCheckup, VersaoDoPacoteDeCheckup } from '@vidacor/api-client';
import { Aviso, Botao, Campo, CampoDeData, Escolha, formatarDinheiro, lerReais } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { CHAVE_PACOTES, dataCurta } from './comum';

interface Linha {
  tuss: string;
  preparo: string;
  porque: string;
}

function reaisParaCampo(centavos: number): string {
  return `${String(Math.floor(centavos / 100))},${String(centavos % 100).padStart(2, '0')}`;
}

/**
 * Pacote novo (sem `pacote`) ou versão nova (gerência). A ordem da lista é a ordem do roteiro. O avulso,
 * a economia e o limite de desconto são conferidos pelo servidor com a tabela do particular.
 */
export function FormularioDoPacote({ pacote, base, aoConcluir, aoFechar }: { pacote: PacoteDeCheckup | null; base: VersaoDoPacoteDeCheckup | null; aoConcluir: (codigo: string) => void; aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const catalogo = useQuery({ queryKey: ['catalogo-de-exames'], queryFn: () => api.catalogo.listar() });
  const [nome, setNome] = useState(base?.nome ?? '');
  const [paraQuem, setParaQuem] = useState(base?.para_quem ?? '');
  const [desde, setDesde] = useState<CalendarDate | null>(null);
  const [preco, setPreco] = useState(base === null ? '' : reaisParaCampo(base.preco_centavos));
  const [motivo, setMotivo] = useState('');
  const [linhas, setLinhas] = useState<Linha[]>(base?.exames.map((e) => ({ tuss: e.tuss, preparo: e.preparo ?? '', porque: e.porque ?? '' })) ?? []);
  const [incluir, setIncluir] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (n: NovoPacoteDeCheckup, chave: string) =>
      pacote === null || base === null ? api.pacotes.criar(n, { idempotencyKey: chave }) : api.pacotes.novaVersao(pacote.codigo, { ...n, regra_versao_vista: pacote.versoes[0]?.regra_versao ?? base.regra_versao }, { idempotencyKey: chave }),
    {
      aoConcluir: async (p) => {
        await cliente.invalidateQueries({ queryKey: CHAVE_PACOTES });
        aoConcluir(p.codigo);
      },
    },
  );
  const nomes = new Map((catalogo.data?.procedimentos ?? []).map((p) => [p.tuss, p.nome]));
  const ativos = (catalogo.data?.procedimentos ?? []).filter((p) => p.ativo && !linhas.some((l) => l.tuss === p.tuss));
  const valor = lerReais(preco);
  const erros = {
    nome: nome.trim().length >= 2 ? null : 'Dê um nome ao pacote.',
    paraQuem: paraQuem.trim().length >= 3 ? null : 'Diga para quem é o pacote.',
    desde: desde === null ? 'Diga quando começa.' : null,
    preco: valor !== null && valor > 0 ? null : 'Valor em reais, ex.: 690,00.',
    motivo: motivo.trim().length >= 5 ? null : 'Diga o motivo (5 letras ou mais).',
    exames: linhas.length >= 2 && linhas.length <= 10 ? null : 'O pacote tem de 2 a 10 exames.',
    linhas: linhas.some((l) => l.preparo.trim().length < 3 || l.porque.trim().length < 3) ? 'Diga o preparo e o porquê da ordem de cada exame.' : null,
  };
  const mudar = (i: number, campo: keyof Linha, v: string) => {
    setLinhas(linhas.map((l, k) => (k === i ? { ...l, [campo]: v } : l)));
  };
  const mover = (i: number, d: number) => {
    const n = [...linhas];
    const [x] = n.splice(i, 1);
    if (x !== undefined) n.splice(i + d, 0, x);
    setLinhas(n);
  };
  const titulo = pacote === null ? 'Novo pacote' : `Nova versão, ${pacote.nome}`;
  if (op.resultado !== null) {
    const v = op.resultado.versoes[0];
    return (
      <Aviso
        nivel="feito"
        titulo={`${op.resultado.nome}: versão ${String(v?.regra_versao ?? '')} publicada; começa em ${v === undefined ? '' : dataCurta(v.vigente_desde)}.`}
        acoes={
          <Botao pequeno onPress={aoFechar}>
            Fechar
          </Botao>
        }
      >
        {v !== undefined && typeof v.avulso_centavos === 'number' ? `Avulso ${formatarDinheiro(v.avulso_centavos)}, pacote ${formatarDinheiro(v.preco_centavos)}, calculados pelo servidor. ` : ''}A versão vigente não mudou; orçamentos já emitidos mantêm o valor.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label={titulo}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (Object.values(erros).every((e) => e === null) && desde !== null && valor !== null)
          op.enviar({
            nome: nome.trim(),
            para_quem: paraQuem.trim(),
            vigente_desde: desde.toString(),
            preco_centavos: valor,
            motivo: motivo.trim(),
            exames: linhas.map((l) => ({ tuss: l.tuss, preparo: l.preparo.trim(), porque: l.porque.trim() })),
          });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        {titulo}
      </h2>
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Campo rotulo="Nome do pacote" value={nome} onChange={setNome} erro={tentou ? erros.nome : null} />
        <Campo rotulo="Para quem é" value={paraQuem} onChange={setParaQuem} erro={tentou ? erros.paraQuem : null} />
        <CampoDeData rotulo="Começa em" valor={desde} aoMudar={setDesde} ajuda={pacote === null ? 'Hoje ou depois' : 'A partir de amanhã'} erro={tentou ? erros.desde : null} />
        <Campo rotulo="Preço do pacote (R$)" value={preco} onChange={setPreco} inputMode="decimal" erro={tentou ? erros.preco : null} />
      </div>
      <fieldset className="col" style={{ gap: 8, border: 0, padding: 0, margin: 0 }}>
        <legend className="t13 b">Exames na ordem do roteiro</legend>
        {linhas.map((l, i) => (
          <div key={l.tuss} className="col" style={{ gap: 6, borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
            <div className="row" style={{ gap: 8 }}>
              <span className="t14 b" style={{ flex: 1, minWidth: 0 }}>
                {i + 1}. {nomes.get(l.tuss) ?? l.tuss}
              </span>
              <Botao
                pequeno
                isDisabled={i === 0}
                onPress={() => {
                  mover(i, -1);
                }}
              >
                Subir
              </Botao>
              <Botao
                pequeno
                isDisabled={i === linhas.length - 1}
                onPress={() => {
                  mover(i, 1);
                }}
              >
                Descer
              </Botao>
              <Botao
                pequeno
                onPress={() => {
                  setLinhas(linhas.filter((_, k) => k !== i));
                }}
              >
                Tirar
              </Botao>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 8 }}>
              <Campo
                rotulo={`Preparo, ${nomes.get(l.tuss) ?? l.tuss}`}
                value={l.preparo}
                onChange={(v) => {
                  mudar(i, 'preparo', v);
                }}
              />
              <Campo
                rotulo={`Por que nesta posição, ${nomes.get(l.tuss) ?? l.tuss}`}
                value={l.porque}
                onChange={(v) => {
                  mudar(i, 'porque', v);
                }}
              />
            </div>
          </div>
        ))}
        <Escolha
          rotulo="Incluir exame"
          valor={incluir}
          vazio="Escolha o exame"
          opcoes={ativos.map((p) => ({ valor: p.tuss, rotulo: `${p.nome} (TUSS ${p.tuss})` }))}
          aoMudar={(v) => {
            if (v !== '') setLinhas([...linhas, { tuss: v, preparo: '', porque: '' }]);
            setIncluir('');
          }}
        />
        {tentou && (erros.exames ?? erros.linhas) !== null && <span className="campo-erro">{erros.exames ?? erros.linhas}</span>}
      </fieldset>
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou ? erros.motivo : null} />
      <div className="t13 mute">O servidor calcula o avulso pela tabela do particular do primeiro dia e só aceita pacote mais barato que o avulso, dentro do limite de desconto.</div>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="publicar o pacote" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Publicar versão
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
    </form>
  );
}
