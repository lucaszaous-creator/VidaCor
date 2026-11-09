import { useQuery } from '@tanstack/react-query';
import type { FiltroDeMensagens, MensagemDoRegistro } from '@vidacor/api-client';
import { Abas, Aviso, Botao, Chip, formatarData, formatarDataHora, Vazio, type CorDoChip } from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaSessao } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { AvisosAutomaticos } from './AvisosAutomaticos';

const MODELO: Record<string, string> = {
  confirmacao: 'Confirmação do horário',
  laudo_pronto: 'Laudo pronto no portal',
  reserva_expirada: 'Reserva sem guia expirou',
  oferta_vaga: 'Vaga da lista de espera',
};

const SITUACAO: Record<string, { texto: string; cor: CorDoChip }> = {
  enviada: { texto: 'Enviada', cor: 'verde' },
  bloqueada_opt_out: { texto: 'Não enviada: paciente desligou', cor: 'cinza' },
  sem_celular: { texto: 'Não enviada: sem celular', cor: 'ambar' },
  modelo_desligado: { texto: 'Não enviada: aviso desligado', cor: 'cinza' },
};

const RESPOSTA: Record<string, string> = { confirmou: 'Confirmou', saiu: 'Pediu para sair' };

/** Dia de amanhã no fuso da clínica (AAAA-MM-DD), para pedir as confirmações. */
export function amanhaNaClinica(agora: Date = new Date()): string {
  const amanha = new Date(agora.getTime() + 24 * 60 * 60 * 1000);
  return new Intl.DateTimeFormat('en-CA', { timeZone: FUSO_DA_CLINICA, year: 'numeric', month: '2-digit', day: '2-digit' }).format(amanha);
}

function Registro({ mensagens }: { mensagens: MensagemDoRegistro[] }) {
  if (mensagens.length === 0) return <Vazio titulo="Nenhuma mensagem nos últimos 7 dias com este filtro." />;
  return (
    <table className="tb sticky">
      <thead>
        <tr>
          <th>Paciente</th>
          <th>Aviso</th>
          <th className="nw">Quando</th>
          <th>Situação</th>
        </tr>
      </thead>
      <tbody>
        {mensagens.map((m) => {
          const s = SITUACAO[m.situacao] ?? { texto: m.situacao, cor: 'cinza' as const };
          return (
            <tr key={m.id}>
              <td>
                <b style={{ display: 'block' }}>{m.nome}</b>
                <span className="t13 mute num">{formatarData(m.nascimento)}</span>
              </td>
              <td className="t14">{MODELO[m.modelo] ?? m.modelo}</td>
              <td className="t14 num nw">{formatarDataHora(m.criado_em, FUSO_DA_CLINICA)}</td>
              <td>
                <Chip cor={s.cor} texto={s.texto} />
                {m.resposta !== null && m.resposta !== undefined && (
                  <span className="t13" style={{ display: 'block', marginTop: 4 }}>
                    {RESPOSTA[m.resposta] ?? m.resposta}
                  </span>
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

/** Comunicação com pacientes (Comunicacao.dc.html), corte do MVP: registro e mensagens automáticas. */
export function TelaComunicacao() {
  const { api } = useDependencias();
  const sessao = useQuery(consultaSessao(api));
  const gerencia = sessao.data?.usuario.perfis.includes('gerente') ?? false;
  const [filtro, setFiltro] = useState<FiltroDeMensagens>('todas');
  const p = useQuery({ queryKey: ['comunicacao', filtro], queryFn: () => api.comunicacao.painel(filtro) });
  const dia = amanhaNaClinica();
  const lembretes = useOperacao(() => api.comunicacao.lembretes(dia));
  const d = p.data;
  return (
    <>
      <CabecalhoDaTela
        titulo="Comunicação com pacientes"
        complemento={
          d === undefined ? undefined : (
            <Chip cor={d.whatsapp_configurado ? 'verde' : 'ambar'} texto={d.whatsapp_configurado ? 'WhatsApp conectado' : 'WhatsApp não configurado: os avisos ficam na fila'} />
          )
        }
      >
        <Botao
          pequeno
          variante="principal"
          ocupado={lembretes.enviando}
          onPress={() => {
            lembretes.enviar(undefined);
          }}
        >
          Pedir confirmação de amanhã
        </Botao>
      </CabecalhoDaTela>
      {lembretes.resultado !== null && (
        <Aviso nivel="feito" titulo={`${String(lembretes.resultado.enfileirados)} confirmações pedidas para ${formatarData(dia)}.`}>
          {lembretes.resultado.enfileirados === 0 ? 'Todos os horários de amanhã já tinham o pedido; ninguém recebe duas vezes.' : 'Saem pelo WhatsApp em seguida; quem responder SIM fica confirmado na agenda.'}
        </Aviso>
      )}
      {lembretes.erro !== null && <ErroDoAto erro={lembretes.erro} ato="pedir as confirmações" />}
      {d !== undefined && (
        <div className="row wrap" style={{ gap: 24 }} aria-label="Hoje">
          {[
            [d.hoje.enviadas, 'enviadas hoje'],
            [d.hoje.confirmaram, 'confirmaram'],
            [d.hoje.sem_celular, 'sem celular'],
            [d.hoje.opt_out + d.hoje.desligadas, 'não enviadas por opção'],
            [d.pacientes_sem_whatsapp, 'pacientes com WhatsApp desligado'],
          ].map(([n, t]) => (
            <span key={String(t)} className="t14 mute">
              <b className="num" style={{ fontSize: 20, color: 'var(--ink)', marginRight: 6 }}>
                {n}
              </b>
              {t}
            </span>
          ))}
        </div>
      )}
      <section className="card pane fill" aria-label="Comunicação">
        <div className="scroll">
          <EstadoDaConsulta oQue="a comunicação" carregando={p.isPending} erro={p.error} aoTentarDeNovo={() => void p.refetch()} />
          {d !== undefined && (
            <Abas
              rotulo="Seções"
              abas={[
                {
                  id: 'enviadas',
                  rotulo: 'Mensagens enviadas',
                  conteudo: (
                    <div className="col" style={{ gap: 10 }}>
                      <div className="tabs" role="group" aria-label="Filtro">
                        {(
                          [
                            ['todas', 'Todas'],
                            ['nao_entregues', 'Não entregues'],
                            ['confirmaram', 'Confirmaram'],
                          ] as const
                        ).map(([v, t]) => (
                          <button
                            key={v}
                            type="button"
                            className={filtro === v ? 'on' : undefined}
                            aria-pressed={filtro === v}
                            onClick={() => {
                              setFiltro(v);
                            }}
                          >
                            {t}
                          </button>
                        ))}
                      </div>
                      <Registro mensagens={d.mensagens} />
                    </div>
                  ),
                },
                {
                  id: 'automaticas',
                  rotulo: 'Mensagens automáticas',
                  conteudo: (
                    <div className="col" style={{ gap: 8 }}>
                      <p className="t13 mute" style={{ margin: 0 }}>
                        Nenhuma mensagem leva nome de exame ou resultado. {gerencia ? 'Ligar e desligar pede o motivo e fica no lastro.' : 'Só a gerência liga e desliga.'}
                      </p>
                      <AvisosAutomaticos avisos={d.automaticas} gerencia={gerencia} />
                    </div>
                  ),
                },
              ]}
            />
          )}
        </div>
      </section>
    </>
  );
}
