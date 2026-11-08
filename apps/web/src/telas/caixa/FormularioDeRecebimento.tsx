import { useQuery } from '@tanstack/react-query';
import { ErroDaApi, type FormaDePagamento, type PedidoRecebimento } from '@vidacor/api-client';
import { Botao, Campo, Escolha, formatarData, formatarHora, formatarPontosBase, lerPercentual } from '@vidacor/design-system';
import { useCallback, useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { consultaAgenda } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { PedidoDeDesconto } from './PedidoDeDesconto';
import { ResultadoDoRecebimento } from './ResultadoDoRecebimento';

const MOTIVOS = ['Tabela social', 'Paciente frequente', 'Funcionário ou familiar'].map((m) => ({ valor: m, rotulo: m }));
const FORMAS: { valor: FormaDePagamento; rotulo: string }[] = [
  { valor: 'pix', rotulo: 'PIX' },
  { valor: 'cartao', rotulo: 'Cartão' },
  { valor: 'dinheiro', rotulo: 'Dinheiro' },
];
const TUSS = /^\d{8}$/;

interface Props {
  data: string;
  aoAbrirCaixa: () => void;
}

/**
 * Recebimento do particular (Caixa.dc.html, FC-CXA). A tela manda só o que foi feito (procedimentos,
 * desconto pedido, forma); o total, o desconto em reais e a tabela vêm do servidor.
 */
export function FormularioDeRecebimento({ data, aoAbrirCaixa }: Props) {
  const { api } = useDependencias();
  const agenda = useQuery(consultaAgenda(api, data, undefined));
  const [atendimentoId, setAtendimentoId] = useState('');
  const [tuss, setTuss] = useState('');
  const [percentual, setPercentual] = useState('');
  const [motivo, setMotivo] = useState('');
  const [autorizacao, setAutorizacao] = useState('');
  const [forma, setForma] = useState<FormaDePagamento | ''>('');
  const [tentou, setTentou] = useState(false);
  const [pedirAutorizacao, setPedirAutorizacao] = useState(false);

  const op = useOperacao((p: PedidoRecebimento, chave: string) => api.recebimentos.criar(p, { idempotencyKey: chave }));

  const aoAutorizar = useCallback((id: string) => {
    setAutorizacao(id);
  }, []);
  const aoEncerrarPedido = useCallback(() => {
    setPedirAutorizacao(false);
    setAutorizacao('');
  }, []);

  const atendidos = (agenda.data?.itens ?? []).filter((i) => typeof i.atendimento_id === 'string');
  const item = atendidos.find((i) => i.atendimento_id === atendimentoId);
  const pontos = percentual.trim() === '' ? 0 : lerPercentual(percentual);
  const codigos = tuss
    .split(/[\s,;]+/)
    .map((t) => t.trim())
    .filter((t) => t !== '');
  const tussOk = codigos.length > 0 && codigos.every((t) => TUSS.test(t));

  const erros = {
    paciente: tentou && item === undefined ? 'Escolha o paciente com check-in feito.' : null,
    tuss: tentou && !tussOk ? 'Digite os códigos TUSS de 8 dígitos, separados por vírgula.' : null,
    percentual: tentou && pontos === null ? 'Digite o percentual, ex.: 5 ou 7,5.' : null,
    motivo: tentou && pontos !== null && pontos > 0 && motivo === '' ? 'Escolha o motivo do desconto.' : null,
    forma: tentou && forma === '' ? 'Escolha a forma de pagamento.' : null,
  };

  function registrar(): void {
    setTentou(true);
    if (item === undefined || !tussOk || pontos === null || forma === '') return;
    if (pontos > 0 && motivo === '') return;
    op.enviar({
      paciente_id: item.paciente.id,
      atendimento_id: atendimentoId,
      itens: codigos.map((t) => ({ tuss: t })),
      desconto:
        pontos > 0
          ? { pontos_base: pontos, motivo, ...(autorizacao.trim() === '' ? {} : { autorizacao_id: autorizacao.trim() }) }
          : null,
      forma,
    });
  }

  if (op.resultado !== null) {
    return (
      <div className="col" style={{ gap: 12 }}>
        <ResultadoDoRecebimento r={op.resultado} />
        <div>
          <Botao
            pequeno
            onPress={() => {
              op.reiniciar();
              setTentou(false);
              setAtendimentoId('');
              setTuss('');
              setPercentual('');
              setMotivo('');
              setAutorizacao('');
              setPedirAutorizacao(false);
            }}
          >
            Novo recebimento
          </Botao>
        </div>
      </div>
    );
  }

  const erro = op.erro;
  const codigo = erro instanceof ErroDaApi ? erro.codigo : '';
  const limite = erro instanceof ErroDaApi ? erro.campo('limite_pontos_base') : undefined;

  return (
    <form
      className="col"
      style={{ gap: 12 }}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        registrar();
      }}
    >
      <Escolha
        rotulo="Paciente (check-in feito hoje)"
        valor={atendimentoId}
        aoMudar={setAtendimentoId}
        vazio="Escolha o paciente"
        opcoes={atendidos.map((i) => ({
          valor: i.atendimento_id ?? '',
          rotulo: `${i.paciente.nome}, ${formatarData(i.paciente.nascimento)}, ${formatarHora(i.inicio, FUSO_DA_CLINICA)}`,
        }))}
        erro={erros.paciente}
      />
      <Campo rotulo="Procedimentos (códigos TUSS)" value={tuss} onChange={setTuss} inputMode="numeric" ajuda="O valor sai da tabela vigente no servidor; não é digitado aqui." erro={erros.tuss} />
      <div className="grid g2" style={{ gap: 10 }}>
        <Campo rotulo="Desconto (%)" value={percentual} onChange={setPercentual} inputMode="decimal" ajuda="Vazio: sem desconto." erro={erros.percentual} />
        <Escolha rotulo="Motivo do desconto" valor={motivo} aoMudar={setMotivo} vazio="Sem desconto" opcoes={MOTIVOS} erro={erros.motivo} />
      </div>
      <Escolha
        rotulo="Forma de pagamento"
        valor={forma}
        aoMudar={(v) => {
          setForma(v as FormaDePagamento | '');
        }}
        vazio="Escolha a forma"
        opcoes={FORMAS}
        erro={erros.forma}
      />
      {erro !== null && (
        <ErroDoAto
          erro={erro}
          ato="registrar o recebimento"
          aoTentarDeNovo={registrar}
          complemento={codigo === 'desconto-acima-do-limite' && typeof limite === 'number' ? `Seu limite é ${formatarPontosBase(limite)}.` : undefined}
          acoes={
            codigo === 'desconto-acima-do-limite' ? (
              <Botao
                pequeno
                variante="principal"
                onPress={() => {
                  setPedirAutorizacao(true);
                  op.limparErro();
                }}
              >
                Pedir autorização à gerência
              </Botao>
            ) : codigo === 'caixa-fechado' ? (
              <Botao pequeno variante="principal" onPress={aoAbrirCaixa}>
                Abrir o caixa
              </Botao>
            ) : undefined
          }
        />
      )}
      {pedirAutorizacao && item !== undefined && pontos !== null && pontos > 0 && (
        <PedidoDeDesconto pacienteId={item.paciente.id} pontos={pontos} motivo={motivo} aoAutorizar={aoAutorizar} aoEncerrar={aoEncerrarPedido} />
      )}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Registrar recebimento
        </Botao>
      </div>
    </form>
  );
}
