import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ErroDaApi, novaChaveDeIdempotencia, type Contingencia, type NivelDeContingencia, type PedidoContingencia } from '@vidacor/api-client';
import {
  Abas,
  Aviso,
  Botao,
  Chip,
  DialogoDeFormulario,
  Escolha,
  formatarDataHora,
  formatarDinheiro,
  ListaDeDados,
  Vazio,
} from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { chaves, consultaContingenciaAberta, consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { FormularioDeFicha, FormularioDeRecibo, type Lancamento } from './Lancamentos';
import { NumerosSemUso, ResolverDivergencia } from './Numeros';

const NIVEIS: { valor: NivelDeContingencia; rotulo: string }[] = [
  { valor: 'externo', rotulo: 'Externo (internet, portal, WhatsApp, TISS)' },
  { valor: 'setor', rotulo: 'Um setor' },
  { valor: 'total', rotulo: 'Total (servidor local fora do ar)' },
];
const MOTIVOS = ['servidor local fora do ar', 'rede interna fora', 'internet fora (portal, WhatsApp, TISS)', 'falta de energia', 'simulado'].map((m) => ({
  valor: m,
  rotulo: m.charAt(0).toLocaleUpperCase('pt-BR') + m.slice(1),
}));

function Declarar({ aoFechar }: { aoFechar: () => void }) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const sessao = useQuery(consultaSessao(api));
  const [nivel, setNivel] = useState('');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  // Gerados uma vez na estação: repetir a declaração não cria outra e guarda a hora original (FC-CTG-02).
  const [chave] = useState(novaChaveDeIdempotencia);
  const [declaradaEm, setDeclaradaEm] = useState<string | null>(null);
  const op = useOperacao((p: PedidoContingencia) => api.contingencias.declarar(p), {
    chaveInicial: chave,
    aoConcluir: (c) => {
      queryClient.setQueryData(chaves.contingenciaAberta, c);
      aoFechar();
    },
  });

  function declarar(): void {
    setTentou(true);
    const usuario = sessao.data?.usuario;
    if (nivel === '' || motivo === '' || usuario === undefined) return;
    const em = declaradaEm ?? new Date().toISOString();
    setDeclaradaEm(em);
    op.enviar({ nivel: nivel as NivelDeContingencia, motivo, responsavel_id: usuario.id, declarada_em: em, chave });
  }

  return (
    <DialogoDeFormulario aberto titulo="Declarar contingência?" rotuloConfirmar="Declarar agora" ocupado={op.enviando} aoFechar={aoFechar} aoConfirmar={declarar}>
      <Escolha rotulo="Nível" valor={nivel} aoMudar={setNivel} vazio="Escolha o nível" opcoes={NIVEIS} erro={tentou && nivel === '' ? 'Escolha o nível.' : null} />
      <Escolha rotulo="Motivo" valor={motivo} aoMudar={setMotivo} vazio="Escolha o motivo" opcoes={MOTIVOS} erro={tentou && motivo === '' ? 'Escolha o motivo.' : null} />
      <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
        Responsável: <b>{sessao.data?.usuario.nome ?? 'carregando'}</b> (você). Todas as estações, o chat da equipe e a coordenação são avisados.
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="declarar a contingência" aoTentarDeNovo={declarar} />}
    </DialogoDeFormulario>
  );
}

function ListaDeLancamentos({ itens }: { itens: Lancamento[] }) {
  if (itens.length === 0) {
    return <Vazio titulo="Nada lançado nesta sessão" detalhe="Cada ficha e recibo lançado aparece aqui, com as divergências abertas." />;
  }
  return (
    <ul style={{ listStyle: 'none', margin: 0, padding: 0 }} aria-label="Lançamentos desta sessão">
      {itens.map((l, i) => {
        let texto: string;
        let chip: { texto: string; cor: 'verde' | 'ambar' };
        const divergencia =
          l.tipo === 'ficha' ? (l.resultado.situacao === 'divergencia-aberta' ? l.resultado.divergencia : null) : l.resultado.divergencia;
        if (l.tipo === 'ficha') {
          if (l.resultado.situacao === 'lancada') {
            texto = `Ficha F-ATD ${l.numero}: atendimento ${l.resultado.atendimento.codigo} de ${l.resultado.atendimento.paciente.nome}. Registrado em contingência.`;
            chip = { texto: 'Lançada', cor: 'verde' };
          } else {
            texto = `Ficha F-ATD ${l.numero}: divergência ${l.resultado.divergencia.codigo} aberta. ${l.resultado.divergencia.descricao ?? ''}`;
            chip = { texto: 'Divergência', cor: 'ambar' };
          }
        } else {
          const r = l.resultado;
          texto = `Recibo F-REC ${l.numero}: papel ${formatarDinheiro(r.valor_papel_centavos)}, tabela ${formatarDinheiro(r.valor_recalculado_centavos)}.${
            r.divergencia === null ? '' : ` Divergência ${r.divergencia.codigo} aberta: ${r.divergencia.descricao ?? ''}`
          }`;
          chip = r.divergencia === null ? { texto: 'Lançado', cor: 'verde' } : { texto: 'Divergência', cor: 'ambar' };
        }
        return (
          <li key={`${l.tipo}-${l.numero}-${String(i)}`} className="li">
            <span className="t14" style={{ flex: 1, minWidth: 0 }}>
              {texto}
            </span>
            <Chip cor={chip.cor} texto={chip.texto} />
            {divergencia !== null && <ResolverDivergencia divergencia={divergencia} />}
          </li>
        );
      })}
    </ul>
  );
}

function Encerrar({ c }: { c: Contingencia }) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const [aberto, setAberto] = useState(false);
  const op = useOperacao((_: null, chave: string) => api.contingencias.encerrar(c.id, { idempotencyKey: chave }), {
    aoConcluir: () => {
      setAberto(false);
      queryClient.setQueryData(chaves.contingenciaAberta, null);
    },
    aoFalhar: () => {
      setAberto(false);
    },
  });
  const pendencias = op.erro instanceof ErroDaApi && op.erro.codigo === 'pendencias' ? op.erro.campo('pendencias') : null;
  const lista = Array.isArray(pendencias) ? pendencias.filter((p): p is string => typeof p === 'string') : [];
  return (
    <div className="col" style={{ gap: 10 }}>
      {op.erro !== null && (
        <ErroDoAto
          erro={op.erro}
          ato="encerrar a contingência"
          complemento={
            lista.length > 0 ? (
              <ul style={{ margin: '6px 0 0', paddingLeft: 18 }} aria-label="Pendências">
                {lista.map((p) => (
                  <li key={p}>{p}</li>
                ))}
              </ul>
            ) : undefined
          }
        />
      )}
      <div className="row">
        <Botao
          onPress={() => {
            setAberto(true);
          }}
        >
          Encerrar contingência
        </Botao>
      </div>
      <DialogoDeFormulario
        aberto={aberto}
        titulo={`Encerrar a contingência ${c.codigo}?`}
        rotuloConfirmar="Encerrar"
        ocupado={op.enviando}
        aoFechar={() => {
          setAberto(false);
        }}
        aoConfirmar={() => {
          op.enviar(null);
        }}
      >
        <p className="t14" style={{ margin: 0 }}>
          Só encerra sem pendências: fichas sem lançamento, divergências abertas ou números da faixa sem situação bloqueiam.
        </p>
      </DialogoDeFormulario>
    </div>
  );
}

/** Contingência (Contingencia.dc.html, FC-CTG): declarar, lançar o papel depois e encerrar sem pendência. */
export function TelaContingencia() {
  const { api } = useDependencias();
  const aberta = useQuery(consultaContingenciaAberta(api));
  const [declarando, setDeclarando] = useState(false);
  const [lancamentos, setLancamentos] = useState<Lancamento[]>([]);
  const c = aberta.data;
  const lancar = (l: Lancamento) => {
    setLancamentos((ls) => [l, ...ls]);
  };

  return (
    <>
      <CabecalhoDaTela titulo="Contingência" complemento={<span className="t14 mute">Se o sistema cair, o atendimento segue no papel e é lançado depois</span>}>
        {c === null && (
          <Botao
            variante="principal"
            onPress={() => {
              setDeclarando(true);
            }}
          >
            Declarar contingência
          </Botao>
        )}
      </CabecalhoDaTela>
      <EstadoDaConsulta
        oQue="a contingência"
        carregando={aberta.isPending}
        erro={aberta.error}
        aoTentarDeNovo={() => void aberta.refetch()}
        semPermissao={{ titulo: 'Você não tem acesso à contingência', detalhe: 'Peça à coordenação.' }}
      />
      {c === null && <Vazio titulo="Nenhuma contingência aberta" detalhe="O atendimento está no sistema. Declare só se algo parar." />}
      {c !== undefined && c !== null && (
        <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.2fr) minmax(0, 1fr)', gap: 16 }}>
          <section className="card pane" aria-label="Lançamento do papel">
            <div className="scroll">
              <Abas
                rotulo="Formulário"
                abas={[
                  { id: 'ficha', rotulo: 'Ficha F-ATD', conteudo: <div style={{ paddingTop: 12 }}><FormularioDeFicha contingenciaId={c.id} aoLancar={lancar} /></div> },
                  { id: 'recibo', rotulo: 'Recibo F-REC', conteudo: <div style={{ paddingTop: 12 }}><FormularioDeRecibo contingenciaId={c.id} aoLancar={lancar} /></div> },
                  { id: 'numeros', rotulo: 'Números sem uso', conteudo: <div style={{ paddingTop: 12 }}><NumerosSemUso contingenciaId={c.id} /></div> },
                ]}
              />
            </div>
          </section>
          <section className="card pane" aria-label="Contingência aberta">
            <div className="scroll col" style={{ gap: 14 }}>
              <div className="row" style={{ gap: 10 }}>
                <h2 className="t15 b">
                  <span className="num">{c.codigo}</span>
                </h2>
                <Chip cor="rosa" texto="Aberta" />
              </div>
              <ListaDeDados
                itens={[
                  { rotulo: 'Nível', valor: NIVEIS.find((n) => n.valor === c.nivel)?.rotulo ?? c.nivel },
                  { rotulo: 'Motivo', valor: c.motivo },
                  { rotulo: 'Declarada', valor: formatarDataHora(c.declarada_em, FUSO_DA_CLINICA) },
                ]}
              />
              <h3 className="t15 b">Lançamentos e divergências</h3>
              <ListaDeLancamentos itens={lancamentos} />
              <Aviso nivel="informativo">
                A lista mostra o que foi lançado nesta estação agora. As pendências de todas as estações aparecem ao tentar encerrar.
              </Aviso>
              <Encerrar c={c} />
            </div>
          </section>
        </div>
      )}
      {declarando && (
        <Declarar
          aoFechar={() => {
            setDeclarando(false);
          }}
        />
      )}
    </>
  );
}
