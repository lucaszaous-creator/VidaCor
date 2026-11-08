import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { AparelhoCadastrado, MudancaDeAparelho, NovoAparelho } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Chip, Escolha, formatarHora, ListaDeDados, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSalas } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const SITUACAO: Record<AparelhoCadastrado['situacao'], { texto: string; cor: 'verde' | 'ambar' | 'cinza' }> = {
  conectado: { texto: 'Conectado', cor: 'verde' },
  atencao: { texto: 'Atenção', cor: 'ambar' },
  desativado: { texto: 'Desativado', cor: 'cinza' },
};

function Detalhe({ a }: { a: AparelhoCadastrado }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((m: MudancaDeAparelho, chave: string) => api.aparelhos.mudar(a.id, m, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['aparelhos'] }),
  });
  const tipo: MudancaDeAparelho['tipo'] = a.ativo ? 'desativar' : 'reativar';
  const motivoOk = motivo.trim().length >= 5;
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <h2 className="t15 b" style={{ margin: 0 }}>
          {a.nome}
        </h2>
        <Chip cor={SITUACAO[a.situacao].cor} texto={SITUACAO[a.situacao].texto} />
      </div>
      <ListaDeDados
        larguraDoRotulo={150}
        itens={[
          { rotulo: 'Sala', valor: a.sala_nome },
          { rotulo: 'AE Title', valor: <span className="num">{a.ae_title}</span> },
          { rotulo: 'Modalidade', valor: a.modalidade },
          { rotulo: 'Listas enviadas hoje', valor: <span className="num">{a.hoje.listas_enviadas}</span> },
          { rotulo: 'Aguardando envio', valor: <span className="num">{a.hoje.listas_pendentes}</span> },
          { rotulo: 'Último envio', valor: a.hoje.ultimo_envio === null || a.hoje.ultimo_envio === undefined ? 'Nenhum hoje' : formatarHora(a.hoje.ultimo_envio, FUSO_DA_CLINICA) },
          { rotulo: 'Versão do cadastro', valor: <span className="num">{a.versao}</span> },
        ]}
      />
      {a.hoje.listas_com_erro > 0 && (
        <Aviso nivel="atencao" titulo={`${String(a.hoje.listas_com_erro)} lista(s) de trabalho com erro de envio.`}>
          O núcleo tenta de novo sozinho. Último erro: {a.hoje.ultimo_erro ?? 'sem detalhe'}. Confira a rede e se o aparelho está ligado.
        </Aviso>
      )}
      <Campo
        rotulo={a.ativo ? 'Motivo para desativar' : 'Motivo para reativar'}
        value={motivo}
        onChange={setMotivo}
        ajuda={a.ativo ? 'Desativado, o aparelho não recebe lista de trabalho; o bipe na sala avisa.' : undefined}
        erro={tentou && !motivoOk ? 'Escreva o motivo (5 letras ou mais).' : null}
      />
      <div aria-live="polite">
        {op.resultado !== null && <Aviso nivel="feito" titulo={`${op.resultado.nome} ${op.resultado.ativo ? 'reativado' : 'desativado'}.`}>Mudança registrada no histórico, versão {op.resultado.versao}.</Aviso>}
        {op.erro !== null && <ErroDoAto erro={op.erro} ato={`${tipo} o aparelho`} />}
      </div>
      <div className="row">
        <Botao
          ocupado={op.enviando}
          onPress={() => {
            setTentou(true);
            if (motivoOk) op.enviar({ tipo, motivo: motivo.trim() });
          }}
        >
          {a.ativo ? 'Desativar aparelho' : 'Reativar aparelho'}
        </Botao>
      </div>
    </div>
  );
}

function Cadastro({ aoFechar }: { aoFechar: () => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const salas = useQuery(consultaSalas(api));
  const [nome, setNome] = useState('');
  const [ae, setAe] = useState('');
  const [modalidade, setModalidade] = useState('');
  const [sala, setSala] = useState('');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoAparelho, chave: string) => api.aparelhos.cadastrar(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: ['aparelhos'] }),
  });
  const aeOk = /^[A-Za-z0-9_]{1,16}$/.test(ae.trim());
  const modOk = /^[A-Za-z]{2,4}$/.test(modalidade.trim());
  const ok = nome.trim().length >= 2 && aeOk && modOk && sala !== '' && motivo.trim().length >= 5;
  if (op.resultado !== null) {
    return (
      <Aviso nivel="feito" titulo={`${op.resultado.nome} cadastrado na ${op.resultado.sala_nome}.`} acoes={<Botao pequeno onPress={aoFechar}>Fechar</Botao>}>
        AE Title {op.resultado.ae_title}. Configure o mesmo AE Title no aparelho para ele receber a lista de trabalho.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Adicionar aparelho"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok) op.enviar({ nome: nome.trim(), ae_title: ae.trim().toUpperCase(), modalidade: modalidade.trim().toUpperCase(), sala_id: sala, motivo: motivo.trim() });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Adicionar aparelho
      </h2>
      <Campo rotulo="Nome do aparelho" value={nome} onChange={setNome} erro={tentou && nome.trim().length < 2 ? 'Diga o nome do aparelho.' : null} />
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Campo rotulo="AE Title" value={ae} onChange={setAe} ajuda="Até 16 letras, números ou _" erro={tentou && !aeOk ? 'Use até 16 letras, números ou _.' : null} />
        <Campo rotulo="Modalidade" value={modalidade} onChange={setModalidade} ajuda="Ex.: US, CT, DX" erro={tentou && !modOk ? 'Use 2 a 4 letras.' : null} />
      </div>
      <Escolha rotulo="Sala" valor={sala} aoMudar={setSala} vazio="Escolha a sala" opcoes={(salas.data ?? []).map((s) => ({ valor: s.id, rotulo: s.nome }))} erro={tentou && sala === '' ? 'Escolha a sala.' : null} />
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Escreva o motivo (5 letras ou mais).' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="cadastrar o aparelho" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Cadastrar aparelho
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
    </form>
  );
}

/**
 * Aparelhos e integração DICOM (Aparelhos.dc.html). Para a TI e a gerência: situação de cada aparelho
 * e a atividade do dia só em números (a TI não vê dado clínico, docs/18).
 */
export function TelaAparelhos() {
  const { api } = useDependencias();
  const painel = useQuery({ queryKey: ['aparelhos'], queryFn: () => api.aparelhos.painel(), refetchInterval: 30_000 });
  const [escolhido, setEscolhido] = useState<string | null>(null);
  const [cadastrando, setCadastrando] = useState(false);
  const p = painel.data;
  const atual = p?.aparelhos.find((a) => a.id === escolhido) ?? null;
  return (
    <>
      <CabecalhoDaTela
        titulo="Aparelhos"
        complemento={
          p === undefined ? undefined : (
            <span className="t14 mute">
              {p.estudos_recebidos_hoje} estudos recebidos hoje, {p.estudos_sem_paciente} sem paciente conciliado
            </span>
          )
        }
      />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gridTemplateRows: 'minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Aparelhos da unidade" style={{ padding: 0 }}>
          <div className="card-h row" style={{ padding: '14px 16px 0', marginBottom: 8 }}>
            <h2 style={{ flex: 1 }}>Aparelhos da unidade</h2>
            <Botao
              pequeno
              variante="principal"
              onPress={() => {
                setCadastrando(true);
                setEscolhido(null);
              }}
            >
              Adicionar aparelho
            </Botao>
          </div>
          <div className="scroll">
            <EstadoDaConsulta oQue="os aparelhos" carregando={painel.isPending} erro={painel.error} aoTentarDeNovo={() => void painel.refetch()} />
            {p?.aparelhos.length === 0 && <Vazio titulo="Nenhum aparelho cadastrado." />}
            <ul style={{ listStyle: 'none', margin: 0, padding: '0 8px 8px' }}>
              {p?.aparelhos.map((a) => (
                <li key={a.id}>
                  <button
                    type="button"
                    className={`linha-paciente${a.id === escolhido ? ' sel' : ''}`}
                    aria-current={a.id === escolhido ? 'true' : undefined}
                    style={{ width: '100%', textAlign: 'left', display: 'flex', gap: 10, alignItems: 'center', padding: '10px 8px', border: 0, background: 'transparent', borderBottom: '1px solid var(--line)', cursor: 'pointer' }}
                    onClick={() => {
                      setEscolhido(a.id);
                      setCadastrando(false);
                    }}
                  >
                    <span style={{ flex: 1, minWidth: 0 }}>
                      <b style={{ display: 'block' }}>{a.nome}</b>
                      <span className="t13 mute">
                        {a.sala_nome}, {a.modalidade}, <span className="num">{a.ae_title}</span>
                      </span>
                    </span>
                    <span className="num t13 mute">{a.hoje.listas_enviadas} listas</span>
                    <Chip cor={SITUACAO[a.situacao].cor} texto={SITUACAO[a.situacao].texto} />
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </section>
        <section className="card pane" aria-label="Detalhe do aparelho">
          <div className="scroll">
            {cadastrando ? (
              <Cadastro
                aoFechar={() => {
                  setCadastrando(false);
                }}
              />
            ) : atual === null ? (
              <Vazio titulo="Escolha um aparelho." detalhe="A conexão, as listas de trabalho do dia e o histórico aparecem aqui." />
            ) : (
              <Detalhe key={atual.id} a={atual} />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
