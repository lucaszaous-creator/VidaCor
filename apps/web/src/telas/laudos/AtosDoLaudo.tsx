import { useQueryClient } from '@tanstack/react-query';
import type { Laudo } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, Campo, Dialogo, DialogoDeFormulario, Escolha, formatarData, formatarDataHora } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { chaves } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar, type Retomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

function useAtualizarLaudo(id: string) {
  const queryClient = useQueryClient();
  return async (l: Laudo) => {
    queryClient.setQueryData(chaves.laudo(id), l);
    await queryClient.invalidateQueries({ queryKey: chaves.filaDeLaudos });
  };
}

interface PropsDaAssinatura {
  laudo: Laudo;
  retomada: Retomada | null;
  /** Grava o rascunho pendente antes de assinar; `false` impede a assinatura. */
  antesDeAssinar: () => Promise<boolean>;
  aoFechar: () => void;
}

/**
 * Assinatura com confirmação nominal (FC-LAU-01, R3): "Assinar o laudo de {nome} ({nascimento})?",
 * PIN do certificado e reautenticação quando o servidor pedir. Depois de assinado, o texto trava.
 */
export function DialogoDeAssinatura({ laudo, retomada, antesDeAssinar, aoFechar }: PropsDaAssinatura) {
  const { api } = useDependencias();
  const reautenticar = useReautenticar();
  const atualizar = useAtualizarLaudo(laudo.id);
  const [pin, setPin] = useState('');
  const [tentou, setTentou] = useState(false);
  const [rascunhoNaoSalvo, setRascunhoNaoSalvo] = useState(false);
  const op = useOperacao((p: string, chave: string) => api.laudos.assinar(laudo.id, { pin: p }, { idempotencyKey: chave }), {
    chaveInicial: retomada?.retomar === 'assinar' ? retomada.chave : undefined,
    aoConcluir: async (l) => {
      await atualizar(l);
      aoFechar();
    },
  });
  const p = laudo.paciente;
  const nasc = formatarData(p.nascimento);

  async function assinar(): Promise<void> {
    setTentou(true);
    if (pin.trim() === '') return;
    const salvo = await antesDeAssinar();
    setRascunhoNaoSalvo(!salvo);
    if (salvo) op.enviar(pin);
  }

  return (
    <DialogoDeFormulario
      aberto
      irreversivel
      titulo={`Assinar o laudo de ${p.nome} (${nasc})?`}
      rotuloConfirmar="Assinar agora"
      ocupado={op.enviando}
      aoFechar={aoFechar}
      aoConfirmar={() => void assinar()}
    >
      <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
        {laudo.exame}, {laudo.codigo}. Depois de assinado o texto fica travado; correção vira nova versão com motivo.
        {laudo.conversar_antes ? ' Marcado para conversar antes: não vai ao portal até você liberar.' : ' Vai ao portal do paciente depois de assinado.'}
      </p>
      {retomada?.retomar === 'assinar' && op.erro === null && (
        <Aviso nivel="informativo" titulo="Identidade confirmada.">
          O texto do laudo está salvo. Digite o PIN do certificado de novo para assinar.
        </Aviso>
      )}
      <Campo
        rotulo="PIN do certificado"
        type="password"
        value={pin}
        onChange={setPin}
        autoComplete="off"
        erro={tentou && pin.trim() === '' ? 'Digite o PIN do certificado.' : null}
        isDisabled={op.enviando}
      />
      {rascunhoNaoSalvo && (
        <Aviso nivel="bloqueante" titulo="O rascunho não foi salvo.">
          Nada foi assinado. O texto continua na tela; confira a conexão e tente de novo.
        </Aviso>
      )}
      {op.erro !== null && (
        <ErroDoAto
          erro={op.erro}
          ato="assinar o laudo"
          naoPreservado="o PIN do certificado"
          aoReautenticar={() => {
            reautenticar('assinar', op.chave());
          }}
          aoTentarDeNovo={() => void assinar()}
        />
      )}
    </DialogoDeFormulario>
  );
}

interface PropsDaRetificacao {
  laudo: Laudo;
  textoVigente: string;
  retomada: Retomada | null;
  aoFechar: () => void;
}

/** Retificação com motivo obrigatório (FC-LAU-05, 06): cria a versão seguinte em rascunho. */
export function DialogoDeRetificacao({ laudo, textoVigente, retomada, aoFechar }: PropsDaRetificacao) {
  const { api } = useDependencias();
  const reautenticar = useReautenticar();
  const atualizar = useAtualizarLaudo(laudo.id);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((m: string, chave: string) => api.laudos.retificar(laudo.id, { texto: textoVigente, motivo: m }, { idempotencyKey: chave }), {
    chaveInicial: retomada?.retomar === 'retificar' ? retomada.chave : undefined,
    aoConcluir: async (l) => {
      await atualizar(l);
      aoFechar();
    },
  });
  const p = laudo.paciente;
  return (
    <DialogoDeFormulario
      aberto
      titulo={`Retificar o laudo de ${p.nome}?`}
      paciente={{ nome: p.nome, nascimento: formatarData(p.nascimento) }}
      rotuloConfirmar="Criar retificação"
      ocupado={op.enviando}
      aoFechar={aoFechar}
      aoConfirmar={() => {
        setTentou(true);
        if (motivo.trim() !== '') op.enviar(motivo.trim());
      }}
    >
      <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
        A versão {laudo.versao_vigente} continua consultável e passa a &quot;substituída&quot; quando a nova versão for assinada. O motivo fica no
        lastro e o portal mostra a indicação de retificação.
      </p>
      {retomada?.retomar === 'retificar' && op.erro === null && (
        <Aviso nivel="informativo" titulo="Identidade confirmada.">
          Digite de novo o motivo: por segurança ele não fica guardado fora da tela.
        </Aviso>
      )}
      <AreaDeTexto
        rotulo="Motivo da retificação"
        value={motivo}
        onChange={setMotivo}
        linhas={2}
        placeholder="Ex.: medida do átrio esquerdo digitada errada"
        erro={tentou && motivo.trim() === '' ? 'Escreva o motivo da retificação.' : null}
        isDisabled={op.enviando}
      />
      {op.erro !== null && (
        <ErroDoAto
          erro={op.erro}
          ato="retificar o laudo"
          naoPreservado="o motivo digitado"
          aoReautenticar={() => {
            reautenticar('retificar', op.chave());
          }}
          aoTentarDeNovo={() => {
            op.enviar(motivo.trim());
          }}
        />
      )}
    </DialogoDeFormulario>
  );
}

/** Liberação no portal depois da conversa com o paciente (FC-LAU-08, R11). */
export function BotaoDeLiberacao({ laudo }: { laudo: Laudo }) {
  const { api } = useDependencias();
  const atualizar = useAtualizarLaudo(laudo.id);
  const [aberto, setAberto] = useState(false);
  const op = useOperacao((_: null, chave: string) => api.laudos.liberar(laudo.id, { idempotencyKey: chave }), {
    aoConcluir: async (l) => {
      setAberto(false);
      await atualizar(l);
    },
    aoFalhar: () => {
      setAberto(false);
    },
  });
  const p = laudo.paciente;
  return (
    <>
      <Botao
        pequeno
        variante="principal"
        onPress={() => {
          setAberto(true);
        }}
      >
        Liberar no portal após a conversa
      </Botao>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="liberar o laudo no portal" />}
      <Dialogo
        aberto={aberto}
        titulo={`Liberar o laudo de ${p.nome} no portal?`}
        paciente={{ nome: p.nome, nascimento: formatarData(p.nascimento) }}
        rotuloConfirmar="Liberar no portal"
        ocupado={op.enviando}
        aoFechar={() => {
          setAberto(false);
        }}
        aoConfirmar={() => {
          op.enviar(null);
        }}
      >
        <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
          Confirme que a conversa com o paciente já aconteceu. O laudo aparece no portal e a liberação grava quem liberou e quando.
        </p>
      </Dialogo>
    </>
  );
}

interface PropsDaEmergencia {
  laudoId: string;
  retomada: Retomada | null;
  aoLiberar: () => void;
}

const MINIMO = 20;

/** Acesso de emergência com justificativa (FC-ACE-03, 04): auditado, avisado e revisado em 48 horas. */
export function AcessoDeEmergencia({ laudoId, retomada, aoLiberar }: PropsDaEmergencia) {
  const { api } = useDependencias();
  const reautenticar = useReautenticar();
  const [justificativa, setJustificativa] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((j: string, chave: string) => api.laudos.acessoDeEmergencia(laudoId, { justificativa: j }, { idempotencyKey: chave }), {
    chaveInicial: retomada?.retomar === 'emergencia' ? retomada.chave : undefined,
    aoConcluir: aoLiberar,
  });
  const curta = justificativa.trim().length < MINIMO;
  return (
    <form
      className="col"
      style={{ gap: 10, maxWidth: 560 }}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (!curta) op.enviar(justificativa.trim());
      }}
    >
      <h3 className="t15 b">Acesso de emergência</h3>
      <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
        Use só em emergência. O acesso vale por 1 hora, fica registrado em destaque, é avisado ao responsável clínico de segurança e entra na
        revisão de 48 horas.
      </p>
      {retomada?.retomar === 'emergencia' && op.erro === null && (
        <Aviso nivel="informativo" titulo="Identidade confirmada.">
          Digite de novo a justificativa: por segurança ela não fica guardada fora da tela.
        </Aviso>
      )}
      <AreaDeTexto
        rotulo="Justificativa"
        value={justificativa}
        onChange={setJustificativa}
        linhas={3}
        ajuda={`Mínimo de ${String(MINIMO)} caracteres. Digitados: ${String(justificativa.trim().length)}.`}
        erro={tentou && curta ? `Escreva pelo menos ${String(MINIMO)} caracteres explicando a emergência.` : null}
        isDisabled={op.enviando}
      />
      {op.erro !== null && (
        <ErroDoAto
          erro={op.erro}
          ato="usar o acesso de emergência"
          naoPreservado="a justificativa digitada"
          aoReautenticar={() => {
            reautenticar('emergencia', op.chave());
          }}
          aoTentarDeNovo={() => {
            op.enviar(justificativa.trim());
          }}
        />
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Usar acesso de emergência
        </Botao>
      </div>
    </form>
  );
}

/** Publicação manual no portal (docs/30 § 7), quando o laudo assinado não foi publicado. Confirmação nominal (R3). */
export function BotaoDePublicacao({ laudo }: { laudo: Laudo }) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const [aberto, setAberto] = useState(false);
  const op = useOperacao((_: null, chave: string) => api.laudos.publicar(laudo.id, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      setAberto(false);
      await queryClient.invalidateQueries({ queryKey: chaves.laudo(laudo.id) });
    },
    aoFalhar: () => {
      setAberto(false);
    },
  });
  const p = laudo.paciente;
  return (
    <>
      <Botao
        pequeno
        variante="principal"
        onPress={() => {
          setAberto(true);
        }}
      >
        Publicar no portal
      </Botao>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="publicar o laudo no portal" />}
      <Dialogo
        aberto={aberto}
        titulo={`Publicar o laudo de ${p.nome} no portal?`}
        paciente={{ nome: p.nome, nascimento: formatarData(p.nascimento) }}
        rotuloConfirmar="Publicar no portal"
        ocupado={op.enviando}
        aoFechar={() => {
          setAberto(false);
        }}
        aoConfirmar={() => {
          op.enviar(null);
        }}
      >
        <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
          O paciente passa a ver o laudo no portal e recebe um aviso pelo canal autorizado.
        </p>
      </Dialogo>
    </>
  );
}

const HORAS = [
  { valor: '24', rotulo: '24 horas' },
  { valor: '48', rotulo: '48 horas' },
  { valor: '72', rotulo: '72 horas' },
];

/** Link do laudo para médico externo (docs/30 § 7): o token aparece uma única vez. */
export function CompartilharLaudo({ laudo }: { laudo: Laudo }) {
  const { api } = useDependencias();
  const [horas, setHoras] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((h: number, chave: string) => api.laudos.criarLink(laudo.id, h, { idempotencyKey: chave }));
  const link = op.resultado;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      noValidate
      aria-label="Compartilhar com médico externo"
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (horas !== '') op.enviar(Number(horas));
      }}
    >
      <h3 className="t15 b">Compartilhar com médico externo</h3>
      {link === null ? (
        <>
          <Escolha
            rotulo="Validade do link"
            valor={horas}
            aoMudar={setHoras}
            vazio="Escolha a validade"
            opcoes={HORAS}
            erro={tentou && horas === '' ? 'Escolha por quanto tempo o link vale.' : null}
          />
          {op.erro !== null && <ErroDoAto erro={op.erro} ato="criar o link" />}
          <div className="row">
            <Botao type="submit" ocupado={op.enviando}>
              Criar link
            </Botao>
          </div>
        </>
      ) : (
        <Aviso nivel="feito" titulo="Link criado.">
          Endereço: <span className="num" style={{ overflowWrap: 'anywhere' }}>{`${window.location.origin}/portal/link/${link.token}`}</span>. Vale até{' '}
          {formatarDataHora(link.expira_em, FUSO_DA_CLINICA)}. Copie agora: o servidor não mostra este endereço de novo.
        </Aviso>
      )}
    </form>
  );
}
