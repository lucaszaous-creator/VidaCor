import type { CalendarDate } from '@internationalized/date';
import type { Guia, PedidoGuia } from '@vidacor/api-client';
import { Botao, BotaoDeIcone, CaixaDeMarcar, Campo, CampoDeData, Escolha, formatarData, ListaDeDados } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

export interface LinhaDeProcedimento {
  chave: number;
  tuss: string;
  quantidade: string;
}

const OPERADORAS = ['Unimed Costa do Sol', 'Bradesco Saúde', 'SulAmérica'].map((o) => ({ valor: o, rotulo: o }));
const TUSS = /^\d{8}$/;

export function tussValidos(linhas: LinhaDeProcedimento[]): string[] {
  return linhas.map((l) => l.tuss.trim()).filter((t) => TUSS.test(t));
}

interface PropsDosProcedimentos {
  linhas: LinhaDeProcedimento[];
  aoMudar: (linhas: LinhaDeProcedimento[]) => void;
  tentou: boolean;
  desabilitado: boolean;
}

/** Procedimentos do pedido médico (TUSS e quantidade), como estão na guia de papel. */
export function ProcedimentosDoPedido({ linhas, aoMudar, tentou, desabilitado }: PropsDosProcedimentos) {
  function mudar(chave: number, campo: 'tuss' | 'quantidade', valor: string): void {
    aoMudar(linhas.map((l) => (l.chave === chave ? { ...l, [campo]: valor } : l)));
  }
  return (
    <div className="col" style={{ gap: 8 }}>
      {linhas.map((l, i) => (
        <div key={l.chave} className="row" style={{ alignItems: 'flex-start', gap: 8 }}>
          <div style={{ flex: '2 1 0' }}>
            <Campo
              rotulo={`Código TUSS ${String(i + 1)}`}
              value={l.tuss}
              onChange={(v) => {
                mudar(l.chave, 'tuss', v);
              }}
              inputMode="numeric"
              erro={tentou && !TUSS.test(l.tuss.trim()) ? 'O código TUSS tem 8 dígitos.' : null}
              isDisabled={desabilitado}
            />
          </div>
          <div style={{ flex: '1 1 0' }}>
            <Campo
              rotulo={`Quantidade ${String(i + 1)}`}
              value={l.quantidade}
              onChange={(v) => {
                mudar(l.chave, 'quantidade', v);
              }}
              inputMode="numeric"
              erro={tentou && !/^[1-9]\d?$/.test(l.quantidade.trim()) ? 'De 1 a 99.' : null}
              isDisabled={desabilitado}
            />
          </div>
          {linhas.length > 1 && (
            <div style={{ paddingTop: 26 }}>
              <BotaoDeIcone
                icone="fechar"
                estilo="fechar"
                rotulo={`Tirar o procedimento ${String(i + 1)}`}
                isDisabled={desabilitado}
                onPress={() => {
                  aoMudar(linhas.filter((x) => x.chave !== l.chave));
                }}
              />
            </div>
          )}
        </div>
      ))}
      <div>
        <Botao
          pequeno
          variante="discreto"
          isDisabled={desabilitado}
          onPress={() => {
            aoMudar([...linhas, { chave: Math.max(0, ...linhas.map((l) => l.chave)) + 1, tuss: '', quantidade: '1' }]);
          }}
        >
          Adicionar procedimento
        </Botao>
      </div>
    </div>
  );
}

interface PropsDaGuia {
  pacienteId: string | null;
  linhas: LinhaDeProcedimento[];
  aoRegistrar: (g: Guia) => void;
}

/** Registro da guia autorizada (FC-AGD-01). O saldo vem do servidor; a tela não edita quantidade autorizada. */
export function RegistroDaGuia({ pacienteId, linhas, aoRegistrar }: PropsDaGuia) {
  const { api } = useDependencias();
  const [operadora, setOperadora] = useState('');
  const [numero, setNumero] = useState('');
  const [carteirinha, setCarteirinha] = useState('');
  const [senha, setSenha] = useState('');
  const [validade, setValidade] = useState<CalendarDate | null>(null);
  const [solicitante, setSolicitante] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((g: PedidoGuia, chave: string) => api.guias.registrar(g, { idempotencyKey: chave }), { aoConcluir: aoRegistrar });

  const vazio = (v: string) => (tentou && v.trim() === '' ? 'Preencha como está na guia.' : null);
  const completo =
    pacienteId !== null &&
    operadora !== '' &&
    [numero, carteirinha, senha, solicitante].every((v) => v.trim() !== '') &&
    validade !== null &&
    tussValidos(linhas).length === linhas.length &&
    linhas.every((l) => /^[1-9]\d?$/.test(l.quantidade.trim()));

  function registrar(): void {
    setTentou(true);
    if (!completo) return;
    op.enviar({
      operadora,
      numero: numero.trim(),
      carteirinha: carteirinha.trim(),
      senha: senha.trim(),
      validade: validade.toString(),
      paciente_id: pacienteId,
      solicitante: solicitante.trim(),
      procedimentos: linhas.map((l) => ({ tuss: l.tuss.trim(), quantidade: Number(l.quantidade.trim()) })),
    });
  }

  return (
    <div className="col" style={{ gap: 10 }}>
      <Escolha rotulo="Operadora" valor={operadora} aoMudar={setOperadora} vazio="Escolha a operadora" opcoes={OPERADORAS} erro={tentou && operadora === '' ? 'Escolha a operadora.' : null} />
      <div className="grid g2" style={{ gap: 10 }}>
        <Campo rotulo="Nº da guia na operadora" value={numero} onChange={setNumero} erro={vazio(numero)} isDisabled={op.enviando} />
        <Campo rotulo="Carteirinha" value={carteirinha} onChange={setCarteirinha} erro={vazio(carteirinha)} isDisabled={op.enviando} />
        <Campo rotulo="Senha de autorização" value={senha} onChange={setSenha} erro={vazio(senha)} autoComplete="off" isDisabled={op.enviando} />
        <CampoDeData rotulo="Senha válida até" valor={validade} aoMudar={setValidade} erro={tentou && validade === null ? 'Digite a validade da senha.' : null} desabilitado={op.enviando} />
      </div>
      <Campo rotulo="Médico solicitante (nome e CRM)" value={solicitante} onChange={setSolicitante} erro={vazio(solicitante)} isDisabled={op.enviando} />
      {tentou && pacienteId === null && <span className="campo-erro">Escolha o paciente antes de registrar a guia.</span>}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar a guia" aoTentarDeNovo={registrar} />}
      <div className="row">
        <Botao pequeno variante="azul" onPress={registrar} ocupado={op.enviando}>
          Registrar guia
        </Botao>
      </div>
    </div>
  );
}

interface PropsDaGuiaRegistrada {
  guia: Guia;
  escolhidos: string[];
  aoEscolher: (tuss: string[]) => void;
  aoTrocar: () => void;
}

/** Guia como o servidor devolveu: saldo por procedimento e escolha dos que vão neste agendamento. */
export function GuiaRegistrada({ guia, escolhidos, aoEscolher, aoTrocar }: PropsDaGuiaRegistrada) {
  return (
    <div className="col" style={{ gap: 10 }}>
      <ListaDeDados
        itens={[
          { rotulo: 'Guia', valor: <span className="num">{guia.numero}</span> },
          { rotulo: 'Operadora', valor: guia.operadora },
          { rotulo: 'Senha válida até', valor: <span className="num">{formatarData(guia.validade)}</span> },
        ]}
      />
      <h3 className="t14 b">Procedimentos autorizados</h3>
      {guia.procedimentos.map((p) => (
        <CaixaDeMarcar
          key={p.tuss}
          rotulo={`TUSS ${p.tuss}`}
          detalhe={`Saldo conferido pelo servidor: ${String(p.saldo)}`}
          marcado={escolhidos.includes(p.tuss)}
          aoMudar={(m) => {
            aoEscolher(m ? [...escolhidos, p.tuss] : escolhidos.filter((t) => t !== p.tuss));
          }}
        />
      ))}
      <div>
        <Botao pequeno variante="discreto" onPress={aoTrocar}>
          Registrar outra guia
        </Botao>
      </div>
    </div>
  );
}
