import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { MedicoDoCorpoClinico, NovoPedidoDoCorpoClinico, PedidoDoCorpoClinico } from '@vidacor/api-client';
import { Abas, Aviso, Botao, Campo, Chip, Escolha, formatarDataHora, ListaDeDados, OpcoesMultiplas, Vazio } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CHAVE_CORPO_CLINICO, resumoDoRegistro, SITUACAO_DO_MEDICO } from './comum';
import { AvisoDoCertificado, ConferenciasDoMedico, SITUACAO_NO_CONSELHO } from './ConferenciasDoMedico';

const UFS = ['AC', 'AL', 'AP', 'AM', 'BA', 'CE', 'DF', 'ES', 'GO', 'MA', 'MT', 'MS', 'MG', 'PA', 'PB', 'PR', 'PE', 'PI', 'RJ', 'RN', 'RS', 'RO', 'RR', 'SC', 'SP', 'SE', 'TO'];

export const MODALIDADES = [
  { valor: 'US', rotulo: 'Ultrassom' },
  { valor: 'DX', rotulo: 'Raio X' },
  { valor: 'CT', rotulo: 'Tomografia' },
  { valor: 'MR', rotulo: 'Ressonância' },
  { valor: 'MG', rotulo: 'Mamografia' },
];

function nomesDasModalidades(lista: string[]): string {
  return lista.length === 0 ? 'Nenhuma' : lista.map((m) => MODALIDADES.find((x) => x.valor === m)?.rotulo ?? m).join(', ');
}

/** O que o pedido muda, em uma linha. */
function oQueMuda(p: PedidoDoCorpoClinico): string {
  if (p.registro !== null && p.registro !== undefined) {
    const r = p.registro;
    const rqe = typeof r.rqe === 'string' ? `, RQE ${r.rqe}` : '';
    const esp = typeof r.especialidade === 'string' ? `, ${r.especialidade}` : '';
    return `registro profissional CRM-${r.uf} ${r.numero}${rqe}${esp}`;
  }
  const h = p.habilitacao;
  return h === null || h === undefined ? p.tipo : `realiza ${nomesDasModalidades(h.realiza)}; lauda ${nomesDasModalidades(h.lauda)}`;
}

function useAtualizar() {
  const cliente = useQueryClient();
  return () => cliente.invalidateQueries({ queryKey: CHAVE_CORPO_CLINICO });
}

/** Pedido aguardando: quem não pediu (gerência) confere e aplica ou devolve, com motivo. */
function Conferencia({ p, euId }: { p: PedidoDoCorpoClinico; euId: string | undefined }) {
  const { api } = useDependencias();
  const reautenticar = useReautenticar();
  const atualizar = useAtualizar();
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const [aprovar, setAprovar] = useState(true);
  const op = useOperacao((a: boolean, chave: string) => api.corpoClinico.decidir(p.id, { aprovar: a, motivo: motivo.trim() }, { idempotencyKey: chave }), {
    aoConcluir: atualizar,
  });
  const motivoOk = motivo.trim().length >= 5;
  const decidir = (a: boolean) => {
    setTentou(true);
    setAprovar(a);
    if (motivoOk) op.enviar(a);
  };
  const fuiEu = euId !== undefined && p.pedido_por_id === euId;
  return (
    <Aviso nivel="atencao" titulo={`Aguarda segunda conferência: ${oQueMuda(p)}.`}>
      <span className="col" style={{ gap: 8 }}>
        <span>
          Pedido por {p.pedido_por} em {formatarDataHora(p.pedido_em, FUSO_DA_CLINICA)}. Motivo: {p.motivo}. Só vale depois da conferência.
        </span>
        {fuiEu ? (
          <span>Quem pediu não confere. Peça a outra pessoa da gerência.</span>
        ) : (
          <>
            <Campo rotulo="Motivo da conferência" value={motivo} onChange={setMotivo} erro={tentou && !motivoOk ? 'Diga o que conferiu (5 letras ou mais).' : null} />
            {op.erro !== null && (
              <ErroDoAto
                erro={op.erro}
                ato="conferir o pedido"
                aoReautenticar={() => {
                  reautenticar('conferir-corpo-clinico', op.chave(), {}, () => {
                    decidir(aprovar);
                  });
                }}
              />
            )}
            <span className="row" style={{ gap: 8 }}>
              <Botao
                pequeno
                variante="principal"
                ocupado={op.enviando}
                onPress={() => {
                  decidir(true);
                }}
              >
                Conferi: aplicar
              </Botao>
              <Botao
                pequeno
                onPress={() => {
                  decidir(false);
                }}
              >
                Devolver sem aplicar
              </Botao>
            </span>
          </>
        )}
      </span>
    </Aviso>
  );
}

/** Formulário de pedido (registro ou habilitação), sempre com motivo e reautenticação. */
function useEnvio(m: MedicoDoCorpoClinico, ato: string) {
  const { api } = useDependencias();
  const reautenticar = useReautenticar();
  const atualizar = useAtualizar();
  const op = useOperacao((p: NovoPedidoDoCorpoClinico, chave: string) => api.corpoClinico.pedir(m.id, p, { idempotencyKey: chave }), { aoConcluir: atualizar });
  const [ultimo, setUltimo] = useState<NovoPedidoDoCorpoClinico | null>(null);
  const enviar = (p: NovoPedidoDoCorpoClinico) => {
    setUltimo(p);
    op.enviar(p);
  };
  const erro =
    op.erro === null ? null : (
      <ErroDoAto
        erro={op.erro}
        ato={ato}
        aoReautenticar={() => {
          reautenticar('pedir-corpo-clinico', op.chave(), {}, () => {
            if (ultimo !== null) op.enviar(ultimo);
          });
        }}
      />
    );
  return { op, enviar, erro };
}

function FormularioDoRegistro({ m }: { m: MedicoDoCorpoClinico }) {
  const { op, enviar, erro } = useEnvio(m, 'pedir a mudança do registro');
  const [uf, setUf] = useState(m.registro?.uf ?? '');
  const [numero, setNumero] = useState(m.registro?.numero ?? '');
  const [rqe, setRqe] = useState(m.registro?.rqe ?? '');
  const [esp, setEsp] = useState(m.registro?.especialidade ?? '');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const numOk = /^\d{1,10}$/.test(numero.trim());
  const rqeOk = rqe.trim() === '' || /^\d{1,10}$/.test(rqe.trim());
  const espOk = rqe.trim() === '' || esp.trim().length >= 3;
  const ok = uf !== '' && numOk && rqeOk && espOk && motivo.trim().length >= 5;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Registro profissional"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok)
          enviar({
            tipo: 'registro',
            registro: {
              uf,
              numero: numero.trim(),
              rqe: rqe.trim() === '' ? null : rqe.trim(),
              especialidade: esp.trim() === '' ? null : esp.trim(),
            },
            motivo: motivo.trim(),
          });
      }}
    >
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '120px minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <Escolha rotulo="UF do CRM" valor={uf} aoMudar={setUf} vazio="UF" opcoes={UFS.map((u) => ({ valor: u, rotulo: u }))} erro={tentou && uf === '' ? 'Escolha a UF.' : null} />
        <Campo rotulo="Número do CRM" value={numero} onChange={setNumero} inputMode="numeric" erro={tentou && !numOk ? 'Só algarismos, até 10.' : null} />
      </div>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '120px minmax(0, 1fr)',
          gap: 10,
          alignItems: 'start',
        }}
      >
        <Campo rotulo="RQE" value={rqe} onChange={setRqe} inputMode="numeric" erro={tentou && !rqeOk ? 'Só algarismos.' : null} />
        <Campo rotulo="Especialidade" value={esp} onChange={setEsp} erro={tentou && !espOk ? 'O RQE é de uma especialidade: diga qual.' : null} />
      </div>
      <Campo rotulo="Motivo do pedido" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Diga o motivo (5 letras ou mais).' : null} />
      <div aria-live="polite">
        {erro}
        {op.resultado !== null && op.erro === null && (
          <Aviso nivel="feito" titulo="Pedido enviado para a conferência.">
            O registro só muda depois que outra pessoa da gerência conferir. Registrado no lastro.
          </Aviso>
        )}
      </div>
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Enviar para conferência
        </Botao>
      </div>
    </form>
  );
}

function FormularioDaHabilitacao({ m }: { m: MedicoDoCorpoClinico }) {
  const { op, enviar, erro } = useEnvio(m, 'pedir a mudança do que realiza e lauda');
  const [realiza, setRealiza] = useState<string[]>(m.habilitacao?.realiza ?? []);
  const [lauda, setLauda] = useState<string[]>(m.habilitacao?.lauda ?? []);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const ok = motivo.trim().length >= 5;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Realiza e lauda"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok)
          enviar({
            tipo: 'habilitacao',
            habilitacao: { realiza, lauda },
            motivo: motivo.trim(),
          });
      }}
    >
      <OpcoesMultiplas rotulo="Realiza na sala" valores={realiza} aoMudar={setRealiza} opcoes={MODALIDADES} />
      <OpcoesMultiplas rotulo="Lauda" valores={lauda} aoMudar={setLauda} opcoes={MODALIDADES} />
      <Campo rotulo="Motivo do pedido" value={motivo} onChange={setMotivo} erro={tentou && !ok ? 'Diga o motivo (5 letras ou mais).' : null} />
      <div aria-live="polite">
        {erro}
        {op.resultado !== null && op.erro === null && (
          <Aviso nivel="feito" titulo="Pedido enviado para a conferência.">
            Vale depois que outra pessoa da gerência conferir. Registrado no lastro.
          </Aviso>
        )}
      </div>
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Enviar para conferência
        </Botao>
      </div>
    </form>
  );
}

const SITUACAO_DO_PEDIDO: Record<PedidoDoCorpoClinico['situacao'], string> = {
  aguardando: 'Aguardando',
  aplicado: 'Aplicado',
  recusado: 'Devolvido',
};

function Historico({ m }: { m: MedicoDoCorpoClinico }) {
  const { api } = useDependencias();
  const h = useQuery({
    queryKey: [...CHAVE_CORPO_CLINICO, m.id, 'historico'],
    queryFn: () => api.corpoClinico.historico(m.id),
  });
  return (
    <div className="col" style={{ gap: 8 }}>
      <EstadoDaConsulta oQue="o histórico" carregando={h.isPending} erro={h.error} aoTentarDeNovo={() => void h.refetch()} />
      {h.data?.length === 0 && <Vazio titulo="Nenhuma mudança registrada." />}
      <ul className="col" style={{ gap: 8, listStyle: 'none', margin: 0, padding: 0 }}>
        {h.data?.map((p) => (
          <li key={p.id} style={{ borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
            <div className="row" style={{ gap: 8 }}>
              <b style={{ flex: 1, minWidth: 0 }}>{oQueMuda(p)}</b>
              <span className="t13 mute">{SITUACAO_DO_PEDIDO[p.situacao]}</span>
            </div>
            <div className="t13 mute">
              Pedido por {p.pedido_por} em {formatarDataHora(p.pedido_em, FUSO_DA_CLINICA)}: {p.motivo}.
              {typeof p.decidido_por === 'string' &&
                typeof p.decidido_em === 'string' &&
                ` Conferido por ${p.decidido_por} em ${formatarDataHora(p.decidido_em, FUSO_DA_CLINICA)}: ${p.motivo_decisao ?? ''}.`}
            </div>
            <div className="t13 mute num">{p.codigo}</div>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Detalhe do médico: pedidos aguardando, cadastro vigente e as abas de mudança e histórico. */
export function DetalheDoMedico({ m }: { m: MedicoDoCorpoClinico }) {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const [aba, setAba] = useState('registro');
  const r = m.registro;
  return (
    <>
      <div className="row" style={{ gap: 10 }}>
        <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>{m.nome}</h2>
        <Chip cor={SITUACAO_DO_MEDICO[m.situacao].cor} texto={SITUACAO_DO_MEDICO[m.situacao].texto} />
      </div>
      {m.pendentes.map((p) => (
        <Conferencia key={p.id} p={p} euId={sessao.data?.usuario.id} />
      ))}
      {r === null || r === undefined ? (
        <Aviso nivel="atencao" titulo="Registro profissional não cadastrado.">
          O laudo impresso sai com o aviso de registro não cadastrado até a conferência do CRM.
        </Aviso>
      ) : null}
      <AvisoDoCertificado c={m.certificado} />
      <ListaDeDados
        larguraDoRotulo={150}
        itens={[
          {
            rotulo: 'CRM',
            valor: r === null || r === undefined ? 'Não cadastrado' : resumoDoRegistro(m),
          },
          {
            rotulo: 'RQE',
            valor: typeof r?.rqe === 'string' ? <span className="num">{r.rqe}</span> : 'Não informado',
          },
          {
            rotulo: 'Consulta ao CFM',
            valor:
              m.consulta_crm == null
                ? 'Não consultado'
                : `${SITUACAO_NO_CONSELHO[m.consulta_crm.situacao]}, ${formatarDataHora(m.consulta_crm.consultado_em, FUSO_DA_CLINICA)}`,
          },
          {
            rotulo: 'Realiza na sala',
            valor: nomesDasModalidades(m.habilitacao?.realiza ?? []),
          },
          {
            rotulo: 'Lauda',
            valor: nomesDasModalidades(m.habilitacao?.lauda ?? []),
          },
          {
            rotulo: 'Conta',
            valor: `${m.codigo}${m.ativo ? '' : ', inativa'}`,
          },
        ]}
      />
      <Abas
        rotulo="Cadastro do médico"
        selecionada={aba}
        aoSelecionar={setAba}
        abas={[
          {
            id: 'registro',
            rotulo: 'Registro profissional',
            conteudo: <FormularioDoRegistro m={m} />,
          },
          {
            id: 'habilitacao',
            rotulo: 'Realiza e lauda',
            conteudo: <FormularioDaHabilitacao m={m} />,
          },
          {
            id: 'conferencias',
            rotulo: 'CFM e certificado',
            conteudo: <ConferenciasDoMedico m={m} />,
          },
          {
            id: 'historico',
            rotulo: 'Histórico',
            conteudo: <Historico m={m} />,
          },
        ]}
      />
    </>
  );
}
