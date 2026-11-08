import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { ContaDetalhe } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, CaixaDeMarcar, Campo, Chip, formatarData, formatarDinheiro } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { EFEITO, SITUACAO_DA_CONTA } from './rotulos';

const DUAS_COLUNAS = 'repeat(2, minmax(0, 1fr))';

export const chavesDoFaturamento = {
  tudo: ['faturamento'] as const,
  contas: ['faturamento', 'contas'] as const,
  conta: (id: string) => ['faturamento', 'contas', id] as const,
  lotes: ['faturamento', 'lotes'] as const,
  lote: (id: string) => ['faturamento', 'lotes', id] as const,
  glosas: ['faturamento', 'glosas'] as const,
  contratos: ['faturamento', 'contratos'] as const,
  auditoria: ['faturamento', 'auditoria'] as const,
};

/** Guia SP/SADT com pendências, itens e versões; correção pelo faturista (versão nova com motivo). */
export function DetalheDaConta({ id, podeCorrigir }: { id: string; podeCorrigir: boolean }) {
  const { api } = useDependencias();
  const consulta = useQuery({ queryKey: chavesDoFaturamento.conta(id), queryFn: () => api.faturamento.conta(id) });
  if (consulta.data === undefined) {
    return <EstadoDaConsulta oQue="a guia" carregando={consulta.isPending} erro={consulta.error} aoTentarDeNovo={() => void consulta.refetch()} />;
  }
  const c = consulta.data;
  const r = c.resumo;
  const situacao = SITUACAO_DA_CONTA[r.situacao];
  const aberta = r.lote_codigo == null && ['em_conferencia', 'pronta', 'com_pendencia'].includes(r.situacao);
  return (
    <>
      <div className="row wrap" style={{ gap: 10 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{r.paciente.nome}</h2>
          <div className="t14 mute">
            Nascimento <span className="num">{formatarData(r.paciente.nascimento)}</span>
          </div>
        </div>
        <Chip cor={situacao.cor} texto={situacao.texto} />
      </div>
      <dl className="grid" style={{ gridTemplateColumns: DUAS_COLUNAS, gap: '6px 16px', margin: 0 }}>
        <Dado rotulo="Guia SP/SADT" valor={`${r.codigo}, operadora ${r.numero_guia_operadora}`} />
        <Dado rotulo="Convênio" valor={r.operadora} />
        <Dado rotulo="Carteirinha" valor={c.carteirinha} />
        <Dado rotulo="Senha" valor={`${c.senha}, válida até ${formatarData(c.validade_senha)}`} />
        <Dado rotulo="Exame em" valor={formatarData(r.data_execucao)} />
        <Dado rotulo="Enviar até" valor={r.enviar_ate == null ? 'Sem contrato' : formatarData(r.enviar_ate)} />
        <Dado rotulo="Solicitante" valor={solicitante(c)} />
        <Dado rotulo="Valor" valor={`${formatarDinheiro(r.valor_total_centavos)}, tabela do contrato`} />
      </dl>
      <section aria-label="Procedimentos" className="col" style={{ gap: 4 }}>
        {c.itens.map((i) => (
          <div key={i.sequencial} className="row t14" style={{ gap: 10 }}>
            <span className="num">{i.tuss}</span>
            <span style={{ flex: 1, minWidth: 0 }}>
              {i.descricao}, {i.quantidade}x
            </span>
            <span className="num">{i.valor_unitario_centavos == null ? 'Sem preço na tabela' : formatarDinheiro(i.valor_unitario_centavos)}</span>
          </div>
        ))}
      </section>
      <section aria-label="Pendências da guia" className="col" style={{ gap: 8 }}>
        <h3 className="t15 b" style={{ margin: 0 }}>
          {c.pendencias.length === 0 ? 'Sem pendências' : 'Pendências'}
        </h3>
        {r.regra_versao != null && <p className="t13 mute" style={{ margin: 0 }}>Conferida com {r.regra_versao}.</p>}
        {c.pendencias.map((p) => (
          <div key={p.regra} className="col" style={{ gap: 2, padding: '8px 12px', borderRadius: 'var(--r3)', background: 'var(--gray-bg)' }}>
            <div className="row wrap" style={{ gap: 8 }}>
              <b className="t14">{p.titulo}</b>
              <Chip cor={EFEITO[p.efeito].cor} texto={EFEITO[p.efeito].texto} />
            </div>
            <span className="t14">{p.detalhe}</span>
            <span className="t13 mute">
              {p.o_que_fazer} Origem: {p.origem}.
            </span>
          </div>
        ))}
      </section>
      {c.historico.length > 0 && (
        <section aria-label="Versões anteriores" className="col" style={{ gap: 2 }}>
          <h3 className="t15 b" style={{ margin: 0 }}>
            Versões anteriores
          </h3>
          {c.historico.map((h) => (
            <span key={h.versao} className="t13 mute">
              Versão {h.versao}: {h.motivo}
            </span>
          ))}
        </section>
      )}
      {podeCorrigir && aberta && <Correcao conta={c} />}
    </>
  );
}

function Dado({ rotulo, valor }: { rotulo: string; valor: string }) {
  return (
    <div style={{ minWidth: 0 }}>
      <dt className="t13 mute">{rotulo}</dt>
      <dd className="t14" style={{ margin: 0 }}>
        {valor}
      </dd>
    </div>
  );
}

function solicitante(c: ContaDetalhe): string {
  const s = c.solicitante;
  return s.numero == null ? `${s.nome}, sem conselho` : `${s.nome}, conselho ${s.conselho ?? ''} ${s.numero}, UF ${s.uf ?? ''}, CBO ${s.cbos ?? ''}`;
}

/** Correção antes do lote: o servidor guarda a versão anterior, aplica e confere de novo. */
function Correcao({ conta }: { conta: ContaDetalhe }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const precisaSolicitante = conta.pendencias.some((p) => p.regra === 'solicitante-incompleto');
  const [motivo, setMotivo] = useState('');
  const [conselho, setConselho] = useState(conta.solicitante.conselho ?? '06');
  const [numero, setNumero] = useState(conta.solicitante.numero ?? '');
  const [uf, setUf] = useState(conta.solicitante.uf ?? '');
  const [cbos, setCbos] = useState(conta.solicitante.cbos ?? '');
  const [cid, setCid] = useState(conta.cid ?? '');
  const [assinatura, setAssinatura] = useState(conta.assinatura_beneficiario);
  const [reaplicar, setReaplicar] = useState(false);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (_: undefined, chave: string) =>
      api.faturamento.corrigir(
        conta.resumo.id,
        {
          motivo: motivo.trim(),
          solicitante: numero.trim() === '' ? null : { conselho, numero: numero.trim(), uf, cbos },
          cid: cid.trim() === '' ? null : cid.trim().toUpperCase(),
          assinatura_beneficiario: assinatura === conta.assinatura_beneficiario ? null : assinatura,
          reaplicar_tabela: reaplicar,
        },
        { idempotencyKey: chave },
      ),
    {
      aoConcluir: async (d) => {
        cliente.setQueryData(chavesDoFaturamento.conta(d.resumo.id), d);
        await cliente.invalidateQueries({ queryKey: chavesDoFaturamento.tudo });
      },
    },
  );
  const motivoOk = motivo.trim().length >= 5;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Corrigir a guia"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (motivoOk) op.enviar(undefined);
      }}
    >
      <h3 className="t15 b" style={{ margin: 0 }}>
        Corrigir a guia
      </h3>
      <p className="t13 mute" style={{ margin: 0 }}>
        A versão atual fica guardada com o motivo e o servidor confere a guia de novo.
      </p>
      {(precisaSolicitante || conta.solicitante.numero != null) && (
        <div className="grid" style={{ gridTemplateColumns: DUAS_COLUNAS, gap: 10 }}>
          <Campo rotulo="Conselho (código TISS)" value={conselho} onChange={setConselho} ajuda="06 = CRM" />
          <Campo rotulo="Número no conselho" value={numero} onChange={setNumero} />
          <Campo rotulo="UF (código IBGE)" value={uf} onChange={setUf} ajuda="33 = RJ" />
          <Campo rotulo="CBO" value={cbos} onChange={setCbos} ajuda="6 dígitos" />
        </div>
      )}
      <div className="grid" style={{ gridTemplateColumns: DUAS_COLUNAS, gap: 10 }}>
        <Campo rotulo="CID-10" value={cid} onChange={setCid} ajuda="Sem ponto, ex.: R51" />
        <div className="col" style={{ gap: 6 }}>
          <CaixaDeMarcar rotulo="Assinatura do beneficiário colhida" marcado={assinatura} aoMudar={setAssinatura} />
          <CaixaDeMarcar rotulo="Reaplicar a tabela vigente na data do exame" marcado={reaplicar} aoMudar={setReaplicar} />
        </div>
      </div>
      <AreaDeTexto rotulo="Motivo da correção" value={motivo} onChange={setMotivo} linhas={2} erro={tentou && !motivoOk ? 'Escreva o motivo (pelo menos 5 letras).' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="corrigir a guia" />}
      {op.resultado !== null && (
        <Aviso nivel="feito" titulo={`Guia corrigida: ${SITUACAO_DA_CONTA[op.resultado.resumo.situacao].texto.toLowerCase()}.`}>
          A versão anterior ficou guardada com o motivo.
        </Aviso>
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Salvar correção
        </Botao>
      </div>
    </form>
  );
}
