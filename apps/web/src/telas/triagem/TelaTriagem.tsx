import type { CalendarDate } from '@internationalized/date';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import type { PedidoTriagem, Triagem } from '@vidacor/api-client';
import {
  AreaDeTexto,
  Aviso,
  BarraDoPaciente,
  Botao,
  Campo,
  CampoDeData,
  Chip,
  formatarData,
  formatarDataHora,
  ListaDeDados,
  SimOuNao,
  type CorDoChip,
} from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { chaves, consultaAtendimento, consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar, useRetomada, type Retomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const rota = getRouteApi('/app/triagem/$atendimentoId');

const RESULTADO: Record<string, { texto: string; cor: CorDoChip; explica: string }> = {
  liberado: { texto: 'Liberado', cor: 'verde', explica: 'Pode seguir para a sala.' },
  liberado_com_ressalvas: { texto: 'Liberado com ressalvas', cor: 'azul', explica: 'Pode seguir para a sala com as ressalvas abaixo.' },
  precisa_aprovacao_medica: {
    texto: 'Precisa de aprovação médica',
    cor: 'ambar',
    explica: 'O exame fica bloqueado na sala até a médica registrar a conduta.',
  },
};

type Respostas = Record<'reacao' | 'gestante' | 'amamentando' | 'metformina', boolean | null>;

const PERGUNTAS: { chave: keyof Respostas; texto: string; detalhe?: string }[] = [
  { chave: 'reacao', texto: 'Já teve reação a contraste iodado?' },
  { chave: 'gestante', texto: 'Existe possibilidade de gravidez?' },
  { chave: 'amamentando', texto: 'Está amamentando?' },
  { chave: 'metformina', texto: 'Usa metformina?' },
];

/** `1,4` ou `1.4` vira `"1.4"` (o formato do docs/30). Só conversão de formato; quem calcula é o servidor. */
function lerCreatinina(texto: string): string | null {
  const t = texto.trim().replace(',', '.');
  return /^\d{1,2}(\.\d{1,2})?$/.test(t) ? t : null;
}

function ResultadoDaTriagem({ t }: { t: Triagem }) {
  const r = RESULTADO[t.resultado] ?? { texto: 'Resultado não reconhecido', cor: 'cinza' as const, explica: 'Avise a TI.' };
  return (
    <div className="col" style={{ gap: 12 }}>
      <div className="row" style={{ gap: 10 }}>
        <Chip cor={r.cor} texto={r.texto} />
        <span className="t14">{r.explica}</span>
      </div>
      <ListaDeDados
        larguraDoRotulo={150}
        itens={[
          {
            rotulo: 'TFG',
            valor: t.tfg_calculada === null || t.tfg_calculada === undefined ? 'Não calculada pelo servidor' : `${String(t.tfg_calculada)} mL/min/1,73 m², calculada pelo servidor (CKD-EPI 2021)`,
          },
          { rotulo: 'Regra aplicada', valor: <span className="num">{t.regra_versao}</span> },
          {
            rotulo: 'Motivos',
            valor: t.motivos.length === 0 ? 'Nenhum' : (
              <ul style={{ margin: 0, paddingLeft: 18 }}>
                {t.motivos.map((m) => (
                  <li key={m}>{m}</li>
                ))}
              </ul>
            ),
          },
          {
            rotulo: 'Conduta médica',
            valor:
              t.conduta === null || t.conduta === undefined
                ? 'Não registrada'
                : `${t.conduta.texto} (registrada em ${formatarDataHora(t.conduta.registrada_em, FUSO_DA_CLINICA)})`,
          },
        ]}
      />
    </div>
  );
}

function Conduta({ atendimentoId, retomada, aoRegistrar }: { atendimentoId: string; retomada: Retomada | null; aoRegistrar: () => void }) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const reautenticar = useReautenticar();
  const [texto, setTexto] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((conduta: string, chave: string) => api.atendimentos.registrarConduta(atendimentoId, { conduta }, { idempotencyKey: chave }), {
    chaveInicial: retomada?.retomar === 'conduta' ? retomada.chave : undefined,
    aoConcluir: async (t) => {
      aoRegistrar();
      queryClient.setQueryData(['triagem', atendimentoId], t);
      await queryClient.invalidateQueries({ queryKey: chaves.atendimento(atendimentoId) });
    },
  });
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (texto.trim() !== '') op.enviar(texto.trim());
      }}
    >
      <h3 className="t15 b">Conduta médica</h3>
      {retomada?.retomar === 'conduta' && op.erro === null && (
        <Aviso nivel="informativo" titulo="Identidade confirmada.">
          Digite de novo a conduta: por segurança ela não fica guardada fora da tela.
        </Aviso>
      )}
      <AreaDeTexto
        rotulo="Conduta"
        value={texto}
        onChange={setTexto}
        linhas={3}
        placeholder="Ex.: prosseguir com hidratação"
        erro={tentou && texto.trim() === '' ? 'Escreva a conduta.' : null}
        isDisabled={op.enviando}
      />
      {op.erro !== null && (
        <ErroDoAto
          erro={op.erro}
          ato="registrar a conduta"
          naoPreservado="a conduta digitada"
          aoReautenticar={() => {
            reautenticar('conduta', op.chave(), {}, () => {
              op.enviar(texto.trim());
            });
          }}
          aoTentarDeNovo={() => {
            op.enviar(texto.trim());
          }}
        />
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Registrar conduta e liberar
        </Botao>
      </div>
    </form>
  );
}

/**
 * Triagem de segurança com contraste (Triagem.dc.html, FC-TRI). A tela só pergunta e mostra: a TFG,
 * o resultado, os motivos e a versão da regra vêm do servidor.
 */
export function TelaTriagem() {
  const { api } = useDependencias();
  const { atendimentoId } = rota.useParams();
  const retomada = useRetomada();
  const sessao = useQuery(consultaSessao(api));
  const atendimento = useQuery(consultaAtendimento(api, atendimentoId));
  const triagemSalva = useQuery({ queryKey: ['triagem', atendimentoId], queryFn: () => null as Triagem | null, enabled: false });
  const [respostas, setRespostas] = useState<Respostas>({ reacao: null, gestante: null, amamentando: null, metformina: null });
  const [creatinina, setCreatinina] = useState('');
  const [dataExame, setDataExame] = useState<CalendarDate | null>(null);
  const [tentou, setTentou] = useState(false);
  const [condutaRegistrada, setCondutaRegistrada] = useState(false);
  const queryClient = useQueryClient();

  const registro = useOperacao((corpo: PedidoTriagem, chave: string) => api.atendimentos.registrarTriagem(atendimentoId, corpo, { idempotencyKey: chave }), {
    aoConcluir: (t) => {
      queryClient.setQueryData(['triagem', atendimentoId], t);
    },
  });

  const estado = (
    <EstadoDaConsulta
      oQue="o atendimento"
      carregando={atendimento.isPending}
      erro={atendimento.error}
      aoTentarDeNovo={() => void atendimento.refetch()}
      semPermissao={{ titulo: 'Você não tem acesso a este atendimento', detalhe: 'A triagem é feita por quem está lotado na sala do exame.' }}
    />
  );
  if (atendimento.data === undefined) {
    return (
      <>
        <CabecalhoDaTela titulo="Triagem de segurança" />
        {estado}
      </>
    );
  }
  const a = atendimento.data;
  const triagem = triagemSalva.data ?? a.triagem ?? null;
  const ehMedico = sessao.data?.usuario.perfis.includes('medico') ?? false;
  const creat = lerCreatinina(creatinina);
  const faltam = PERGUNTAS.filter((p) => respostas[p.chave] === null);

  function salvar(): void {
    setTentou(true);
    if (faltam.length > 0 || creat === null || dataExame === null) {
      return;
    }
    registro.enviar({
      reacao_previa_contraste: respostas.reacao === true,
      gestante: respostas.gestante === true,
      amamentando: respostas.amamentando === true,
      usa_metformina: respostas.metformina === true,
      creatinina_mg_dl: creat,
      creatinina_data: dataExame.toString(),
    });
  }

  return (
    <>
      <CabecalhoDaTela titulo="Triagem de segurança" complemento={<span className="t14 mute">Contraste iodado, antes de entrar na sala</span>} />
      <BarraDoPaciente nome={a.paciente.nome} nascimento={formatarData(a.paciente.nascimento)} resumo={`${a.codigo}, accession ${a.accession}`} />
      <div className="fill" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.2fr) minmax(0, 1fr)', gap: 16 }}>
        <section className="card pane" aria-label="Perguntas da triagem">
          <div className="scroll">
            <form
              noValidate
              onSubmit={(ev) => {
                ev.preventDefault();
                salvar();
              }}
              className="col"
              style={{ gap: 4 }}
            >
              {PERGUNTAS.map((p) => (
                <SimOuNao
                  key={p.chave}
                  pergunta={p.texto}
                  valor={respostas[p.chave]}
                  aoMudar={(v) => {
                    setRespostas((r) => ({ ...r, [p.chave]: v }));
                  }}
                  erro={tentou && respostas[p.chave] === null ? 'Responda sim ou não.' : null}
                  desabilitado={registro.enviando}
                />
              ))}
              <h3 className="t15 b" style={{ marginTop: 12 }}>
                Função renal
              </h3>
              <div className="row wrap" style={{ alignItems: 'flex-start' }}>
                <div style={{ flex: '1 1 180px' }}>
                  <Campo
                    rotulo="Creatinina (mg/dL)"
                    value={creatinina}
                    onChange={setCreatinina}
                    inputMode="decimal"
                    ajuda="Como no laudo, ex.: 1,4"
                    erro={tentou && creat === null ? 'Digite a creatinina com até duas casas, ex.: 1,4.' : null}
                    isDisabled={registro.enviando}
                  />
                </div>
                <div style={{ flex: '1 1 180px' }}>
                  <CampoDeData
                    rotulo="Data do exame de creatinina"
                    valor={dataExame}
                    aoMudar={setDataExame}
                    erro={tentou && dataExame === null ? 'Digite a data do exame.' : null}
                    desabilitado={registro.enviando}
                  />
                </div>
              </div>
              <p className="t13 mute" style={{ margin: '4px 0 8px' }}>
                A TFG não é digitada: o servidor calcula com a idade e o sexo do cadastro.
              </p>
              {registro.erro !== null && <ErroDoAto erro={registro.erro} ato="salvar a triagem" aoTentarDeNovo={salvar} />}
              <div className="row">
                <Botao type="submit" variante="principal" ocupado={registro.enviando}>
                  Salvar triagem
                </Botao>
              </div>
            </form>
          </div>
        </section>
        <section className="card pane" aria-label="Resultado da triagem">
          <div className="card-h">
            <h2>Resultado</h2>
          </div>
          <div className="scroll col" style={{ gap: 14 }}>
            {triagem === null ? (
              <p className="t14 mute" style={{ margin: 0 }}>
                Ainda sem triagem registrada. O resultado aparece aqui depois de salvar.
              </p>
            ) : (
              <ResultadoDaTriagem t={triagem} />
            )}
            {condutaRegistrada && (
              <Aviso nivel="feito" titulo="Conduta registrada.">
                O exame foi liberado. A conduta, o seu nome, o horário e a versão da regra ficaram no lastro.
              </Aviso>
            )}
            {triagem !== null &&
              !triagem.exame_liberado &&
              (triagem.conduta ?? null) === null &&
              (ehMedico ? (
                <Conduta
                  atendimentoId={atendimentoId}
                  retomada={retomada}
                  aoRegistrar={() => {
                    setCondutaRegistrada(true);
                  }}
                />
              ) : (
                <Aviso nivel="atencao" titulo="Exame bloqueado até a conduta médica.">
                  A aprovação foi pedida à médica com vínculo de cuidado. Ela registra a conduta nesta tela.
                </Aviso>
              ))}
          </div>
        </section>
      </div>
    </>
  );
}
