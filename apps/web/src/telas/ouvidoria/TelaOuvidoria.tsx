import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { CanalDeManifestacao, Manifestacao, NovaManifestacao, PacienteResumo, TipoDeManifestacao } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, CaixaDeMarcar, Campo, Chip, Escolha, formatarData, ListaDeDados, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { EscolhaDePaciente } from '../../componentes/EscolhaDePaciente';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

const TIPOS: { valor: TipoDeManifestacao; rotulo: string }[] = [
  { valor: 'reclamacao', rotulo: 'Reclamação' },
  { valor: 'elogio', rotulo: 'Elogio' },
  { valor: 'sugestao', rotulo: 'Sugestão' },
  { valor: 'denuncia', rotulo: 'Denúncia' },
  { valor: 'lgpd', rotulo: 'Pedido LGPD' },
];
const CANAIS: { valor: CanalDeManifestacao; rotulo: string }[] = [
  { valor: 'presencial', rotulo: 'Presencial' },
  { valor: 'telefone', rotulo: 'Telefone' },
  { valor: 'email', rotulo: 'E-mail' },
  { valor: 'portal', rotulo: 'Portal' },
];
const NOME_DO_TIPO: Record<string, string> = Object.fromEntries(TIPOS.map((t) => [t.valor, t.rotulo]));

function Registro() {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [tipo, setTipo] = useState<string>('reclamacao');
  const [canal, setCanal] = useState<string>('presencial');
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [nome, setNome] = useState('');
  const [anonima, setAnonima] = useState(false);
  const [relato, setRelato] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovaManifestacao, chave: string) => api.ouvidoria.registrar(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['ouvidoria'] }),
  });
  const podeAnonima = tipo === 'denuncia';
  const identificado = (podeAnonima && anonima) || paciente !== null || (tipo !== 'lgpd' && nome.trim().length >= 2);
  const ok = identificado && relato.trim().length >= 10;
  if (op.resultado !== null) {
    return (
      <Aviso nivel="feito" titulo={`Manifestação registrada: protocolo ${op.resultado.codigo}.`} acoes={<Botao pequeno onPress={op.reiniciar}>Registrar outra</Botao>}>
        Prazo de resposta até {formatarData(op.resultado.prazo_em)}, calculado pelo servidor conforme o tipo. Informe o protocolo à pessoa.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Registrar manifestação"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok)
          op.enviar({
            tipo: tipo as TipoDeManifestacao,
            canal: canal as CanalDeManifestacao,
            paciente_id: podeAnonima && anonima ? null : (paciente?.id ?? null),
            nome_informado: podeAnonima && anonima ? null : paciente === null && nome.trim() !== '' ? nome.trim() : null,
            anonima: podeAnonima && anonima,
            relato: relato.trim(),
          });
      }}
    >
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Escolha rotulo="Tipo" valor={tipo} aoMudar={setTipo} opcoes={TIPOS} />
        <Escolha rotulo="Canal" valor={canal} aoMudar={setCanal} opcoes={CANAIS} />
      </div>
      {podeAnonima && <CaixaDeMarcar rotulo="Denúncia anônima (sem identificar quem denuncia)" marcado={anonima} aoMudar={setAnonima} />}
      {!(podeAnonima && anonima) && (
        <>
          <EscolhaDePaciente rotulo="Paciente do cadastro" paciente={paciente} aoEscolher={setPaciente} />
          {paciente === null && tipo !== 'lgpd' && <Campo rotulo="Ou nome de quem se manifestou" value={nome} onChange={setNome} ajuda="Acompanhante, familiar ou quem não é paciente." />}
          {tentou && !identificado && <span className="campo-erro">{tipo === 'lgpd' ? 'Pedido LGPD é do titular: escolha o paciente do cadastro.' : 'Escolha o paciente ou informe o nome.'}</span>}
        </>
      )}
      <AreaDeTexto rotulo="O que a pessoa relatou" value={relato} onChange={setRelato} linhas={4} erro={tentou && relato.trim().length < 10 ? 'Escreva o relato (10 letras ou mais).' : null} />
      <p className="t13 mute" style={{ margin: 0 }}>
        O prazo é calculado pelo servidor conforme o tipo.
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar a manifestação" />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Registrar e gerar protocolo
        </Botao>
      </div>
    </form>
  );
}

function Detalhe({ m: daLista }: { m: Manifestacao }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [resposta, setResposta] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((_: undefined, chave: string) => api.ouvidoria.responder(daLista.id, resposta.trim(), { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['ouvidoria'] }),
  });
  // A resposta do servidor vale na hora, sem esperar a lista recarregar (a tela nunca mente).
  const m = op.resultado ?? daLista;
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {NOME_DO_TIPO[m.tipo] ?? m.tipo} {m.codigo}
        </h2>
        <Chip cor={m.situacao === 'respondida' ? 'verde' : m.vencida ? 'rosa' : 'ambar'} texto={m.situacao === 'respondida' ? 'Respondida' : m.vencida ? 'Prazo vencido' : 'Aberta'} />
      </div>
      <ListaDeDados
        larguraDoRotulo={110}
        itens={[
          { rotulo: 'Quem', valor: m.anonima ? 'Anônima' : `${m.quem ?? ''}${m.nascimento === null || m.nascimento === undefined ? '' : `, nascimento ${formatarData(m.nascimento)}`}` },
          { rotulo: 'Recebida', valor: `${formatarData(m.recebida_em)}, ${m.canal}` },
          { rotulo: 'Prazo', valor: formatarData(m.prazo_em) },
        ]}
      />
      {m.tipo === 'denuncia' && <Aviso nivel="informativo" titulo="Denúncia sigilosa.">Vista só pela ouvidoria e pela diretoria; quem denunciou acompanha pelo protocolo.</Aviso>}
      <p className="t14" style={{ margin: 0, whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>
        {m.relato}
      </p>
      {m.situacao === 'respondida' ? (
        <Aviso nivel="feito" titulo="Resposta enviada.">
          <span style={{ whiteSpace: 'pre-wrap' }}>{m.resposta}</span>
        </Aviso>
      ) : (
        <>
          <AreaDeTexto rotulo="Resposta" value={resposta} onChange={setResposta} linhas={4} ajuda="Diga o que aconteceu, o que foi feito e o que muda." erro={tentou && resposta.trim().length < 20 ? 'Escreva a resposta (20 letras ou mais).' : null} />
          {op.erro !== null && <ErroDoAto erro={op.erro} ato="enviar a resposta" />}
          <div className="row">
            <Botao
              variante="principal"
              ocupado={op.enviando}
              onPress={() => {
                setTentou(true);
                if (resposta.trim().length >= 20) op.enviar(undefined);
              }}
            >
              Enviar resposta
            </Botao>
          </div>
        </>
      )}
    </div>
  );
}

function Painel() {
  const { api } = useDependencias();
  const p = useQuery({ queryKey: ['ouvidoria'], queryFn: () => api.ouvidoria.painel() });
  const [escolhida, setEscolhida] = useState<string | null>(null);
  const [registrando, setRegistrando] = useState(false);
  const atual = p.data?.manifestacoes.find((m) => m.id === escolhida) ?? null;
  return (
    <>
      <CabecalhoDaTela
        titulo="Ouvidoria"
        complemento={
          p.data === undefined ? undefined : (
            <span className="t14 mute">
              {p.data.abertas} abertas, {p.data.vencem_hoje} vencem hoje
              {p.data.no_prazo_percentual === null || p.data.no_prazo_percentual === undefined ? '' : `, ${String(p.data.no_prazo_percentual)}% respondidas no prazo em 30 dias`}
            </span>
          )
        }
      >
        <Botao
          pequeno
          variante="principal"
          onPress={() => {
            setRegistrando(true);
            setEscolhida(null);
          }}
        >
          Registrar manifestação
        </Botao>
      </CabecalhoDaTela>
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1.2fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Manifestações" style={{ padding: 0 }}>
          <div className="scroll">
            <EstadoDaConsulta oQue="as manifestações" carregando={p.isPending} erro={p.error} aoTentarDeNovo={() => void p.refetch()} />
            {p.data?.manifestacoes.length === 0 && (
              <div style={{ padding: 16 }}>
                <Vazio titulo="Nada aberto." detalhe="As respondidas ficam no histórico." />
              </div>
            )}
            <ul style={{ listStyle: 'none', margin: 0, padding: 8 }}>
              {p.data?.manifestacoes.map((m) => (
                <li key={m.id}>
                  <button
                    type="button"
                    aria-current={m.id === escolhida ? 'true' : undefined}
                    style={{ width: '100%', textAlign: 'left', display: 'flex', gap: 10, alignItems: 'center', padding: '10px 8px', border: 0, background: m.id === escolhida ? 'var(--line)' : 'transparent', borderBottom: '1px solid var(--line)', cursor: 'pointer', font: 'inherit', color: 'inherit' }}
                    onClick={() => {
                      setEscolhida(m.id);
                      setRegistrando(false);
                    }}
                  >
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <b style={{ display: 'block' }}>
                        {NOME_DO_TIPO[m.tipo] ?? m.tipo}, {m.anonima ? 'anônima' : (m.quem ?? '')}
                      </b>
                      <span className="t13 mute">
                        {m.codigo}, prazo {formatarData(m.prazo_em)}
                      </span>
                    </span>
                    <Chip cor={m.situacao === 'respondida' ? 'verde' : m.vencida ? 'rosa' : 'ambar'} texto={m.situacao === 'respondida' ? 'Respondida' : m.vencida ? 'Vencida' : 'Aberta'} />
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe da manifestação">
          <div className="scroll">{registrando ? <Registro /> : atual === null ? <Vazio titulo="Escolha uma manifestação." /> : <Detalhe key={atual.id} m={atual} />}</div>
        </section>
      </div>
    </>
  );
}

/** Ouvidoria (Ouvidoria.dc.html): a gerência vê e responde; a recepção só registra. */
export function TelaOuvidoria() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  if (sessao.data === undefined) return null;
  if (sessao.data.usuario.perfis.includes('gerente')) return <Painel />;
  return (
    <>
      <CabecalhoDaTela titulo="Registrar manifestação" complemento={<span className="t14 mute">A ouvidoria responde dentro do prazo.</span>} />
      <section className="card pane" aria-label="Registro" style={{ maxWidth: 720 }}>
        <div className="scroll">
          <Registro />
        </div>
      </section>
    </>
  );
}
