import type { CalendarDate } from '@internationalized/date';
import { useQueryClient } from '@tanstack/react-query';
import type { AtendimentoDetalhadoFase1, CorrecaoCadastro, Reimpressao } from '@vidacor/api-client';
import {
  AreaDeTexto,
  Aviso,
  Botao,
  CampoDeData,
  DialogoDeFormulario,
  Escolha,
  EtiquetaPulseira,
  formatarData,
} from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { chaves } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar, type Retomada } from '../../reautenticacao';

const MOTIVOS = [
  { valor: 'rasgou', rotulo: 'Rasgou ou apagou' },
  { valor: 'dado impresso errado', rotulo: 'Dado impresso errado' },
  { valor: 'paciente perdeu', rotulo: 'Paciente perdeu' },
  { valor: 'correção de cadastro', rotulo: 'Correção de cadastro' },
];

/** Imprime só a pré-visualização da pulseira (CSS de impressão `.etq-impressao`). */
export function BotaoImprimir({ rotulo = 'Imprimir pulseira' }: { rotulo?: string }) {
  const { imprimir } = useDependencias();
  return (
    <Botao
      pequeno
      onPress={() => {
        (imprimir ?? (() => { window.print(); }))();
      }}
    >
      {rotulo}
    </Botao>
  );
}

interface PropsDaReimpressao {
  atendimento: AtendimentoDetalhadoFase1;
  /** Etiqueta a reimprimir: a válida, ou a última invalidada pela correção de cadastro. */
  codigo: string;
  aberto: boolean;
  motivoInicial?: string;
  aoFechar: () => void;
  aoConcluir: (r: Reimpressao) => void;
}

/** Reimpressão com motivo obrigatório (FC-ETQ-01, 02): a etiqueta atual deixa de valer. */
export function DialogoDeReimpressao({ atendimento, codigo, aberto, motivoInicial = '', aoFechar, aoConcluir }: PropsDaReimpressao) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const [motivo, setMotivo] = useState(motivoInicial);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((m: string, chave: string) => api.etiquetas.reimprimir(codigo, { motivo: m }, { idempotencyKey: chave }), {
    aoConcluir: async (r) => {
      aoConcluir(r);
      await queryClient.invalidateQueries({ queryKey: chaves.atendimento(atendimento.id) });
    },
  });
  const p = atendimento.paciente;
  return (
    <DialogoDeFormulario
      aberto={aberto}
      irreversivel
      titulo={`Reimprimir a pulseira de ${p.nome}?`}
      paciente={{ nome: p.nome, nascimento: formatarData(p.nascimento) }}
      rotuloConfirmar="Reimprimir e invalidar a atual"
      ocupado={op.enviando}
      aoFechar={() => {
        op.limparErro();
        aoFechar();
      }}
      aoConfirmar={() => {
        setTentou(true);
        if (motivo !== '') {
          op.enviar(motivo);
        }
      }}
    >
      <Escolha
        rotulo="Motivo"
        valor={motivo}
        aoMudar={setMotivo}
        vazio="Escolha o motivo"
        opcoes={MOTIVOS}
        erro={tentou && motivo === '' ? 'Escolha o motivo da reimpressão.' : null}
      />
      <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
        A pulseira <b className="num">{codigo}</b> deixa de valer. Se for bipada depois, o sistema bloqueia e avisa.
      </p>
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="reimprimir a pulseira" aoTentarDeNovo={() => { op.enviar(motivo); }} />}
    </DialogoDeFormulario>
  );
}

/** Resultado da reimpressão: a nova válida e a anterior invalidada, lado a lado. */
export function ResultadoDaReimpressao({ r, atendimento }: { r: Reimpressao; atendimento: AtendimentoDetalhadoFase1 }) {
  const p = atendimento.paciente;
  return (
    <div className="col" style={{ gap: 10 }}>
      <Aviso nivel="feito" titulo="Pulseira reimpressa.">
        Nova pulseira <b className="num">{r.nova.codigo}</b>. A anterior foi invalidada com o motivo &quot;{r.invalidada.motivo_invalidacao}&quot;.
      </Aviso>
      <div className="etq-impressao">
        <EtiquetaPulseira nome={p.nome} nascimento={formatarData(p.nascimento)} codigo={r.nova.codigo} accession={atendimento.accession} situacao="valida" />
      </div>
      <EtiquetaPulseira
        nome={p.nome}
        nascimento={formatarData(p.nascimento)}
        codigo={r.invalidada.codigo}
        situacao="invalidada"
        motivo={r.invalidada.motivo_invalidacao}
        substituidaPor={r.invalidada.substituida_por}
      />
    </div>
  );
}

interface PropsDaCorrecao {
  atendimento: AtendimentoDetalhadoFase1;
  aberto: boolean;
  /** Volta da reautenticação: a mesma chave, e o aviso de que os campos precisam ser digitados de novo. */
  retomada: Retomada | null;
  agendamentoId: string;
  aoFechar: () => void;
  aoConcluir: (c: CorrecaoCadastro) => void;
}

/**
 * Correção do nascimento com motivo e reautenticação (FC-ETQ-04). O servidor invalida as etiquetas
 * e a tela pede a reimpressão.
 */
export function DialogoDeCorrecao({ atendimento, aberto, retomada, agendamentoId, aoFechar, aoConcluir }: PropsDaCorrecao) {
  const { api } = useDependencias();
  const reautenticar = useReautenticar();
  const [nascimento, setNascimento] = useState<CalendarDate | null>(null);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (v: { nascimento: string; motivo: string }, chave: string) =>
      api.pacientes.corrigir(atendimento.paciente.id, v, { idempotencyKey: chave }),
    { chaveInicial: retomada?.chave, aoConcluir },
  );
  const p = atendimento.paciente;
  const erroNasc = tentou && nascimento === null ? 'Digite o nascimento correto: dia, mês e ano.' : null;
  const erroMotivo = tentou && motivo.trim() === '' ? 'Escreva o motivo da correção.' : null;
  return (
    <DialogoDeFormulario
      aberto={aberto}
      irreversivel
      titulo={`Corrigir o nascimento de ${p.nome}?`}
      paciente={{ nome: p.nome, nascimento: formatarData(p.nascimento) }}
      rotuloConfirmar="Corrigir cadastro"
      ocupado={op.enviando}
      aoFechar={() => {
        op.limparErro();
        aoFechar();
      }}
      aoConfirmar={() => {
        setTentou(true);
        if (nascimento !== null && motivo.trim() !== '') {
          op.enviar({ nascimento: nascimento.toString(), motivo: motivo.trim() });
        }
      }}
    >
      {retomada !== null && op.erro === null && (
        <Aviso nivel="informativo" titulo="Identidade confirmada.">
          Digite de novo o nascimento e o motivo: por segurança eles não ficam guardados fora da tela.
        </Aviso>
      )}
      <CampoDeData rotulo="Nascimento correto" valor={nascimento} aoMudar={setNascimento} erro={erroNasc} desabilitado={op.enviando} />
      <AreaDeTexto rotulo="Motivo" value={motivo} onChange={setMotivo} erro={erroMotivo} linhas={2} isDisabled={op.enviando} />
      <p className="t14" style={{ margin: 0, lineHeight: 1.5 }}>
        A correção cria uma nova versão do cadastro e invalida todas as etiquetas válidas do paciente. Depois, reimprima a pulseira.
      </p>
      {op.erro !== null && (
        <ErroDoAto
          erro={op.erro}
          ato="corrigir o cadastro"
          naoPreservado="o nascimento e o motivo digitados"
          aoReautenticar={() => {
            reautenticar('corrigir', op.chave(), { agendamento: agendamentoId });
          }}
          aoTentarDeNovo={() => {
            if (nascimento !== null) op.enviar({ nascimento: nascimento.toString(), motivo: motivo.trim() });
          }}
        />
      )}
    </DialogoDeFormulario>
  );
}
