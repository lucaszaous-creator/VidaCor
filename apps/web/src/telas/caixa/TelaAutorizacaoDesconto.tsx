import { useQuery } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import type { PacienteResumo, PedidoAutorizacaoDesconto } from '@vidacor/api-client';
import {
  Aviso,
  Botao,
  Busca,
  Campo,
  Escolha,
  formatarData,
  formatarDataHora,
  formatarPontosBase,
  lerPercentual,
} from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { LinhaDoPaciente } from '../../componentes/LinhaDoPaciente';
import { consultaPacientes } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar, useRetomada } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';

const rota = getRouteApi('/app/caixa/autorizacao-desconto');
const MOTIVOS = ['Tabela social', 'Paciente frequente', 'Funcionário ou familiar'].map((m) => ({ valor: m, rotulo: m }));

/**
 * Autorização de desconto acima do limite (FC-CXA-05), pela gerência, com reautenticação. Abre pelo
 * endereço que o caixa envia (paciente, percentual e quem pediu, só por identificador). A gerente
 * confirma o paciente pelo nome e nascimento antes de autorizar.
 */
export function TelaAutorizacaoDesconto() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const retomada = useRetomada();
  const reautenticar = useReautenticar();
  const [texto, setTexto] = useState('');
  const [paciente, setPaciente] = useState<PacienteResumo | null>(null);
  const [percentual, setPercentual] = useState(busca.pontos === undefined ? '' : formatarPontosBase(busca.pontos).replace('%', ''));
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const pontos = lerPercentual(percentual);
  const pacientes = useQuery({ ...consultaPacientes(api, texto.trim()), enabled: texto.trim().length >= 3 && paciente === null });
  const op = useOperacao((p: PedidoAutorizacaoDesconto, chave: string) => api.autorizacoesDeDesconto.criar(p, { idempotencyKey: chave }), {
    chaveInicial: retomada?.retomar === 'autorizar' ? retomada.chave : undefined,
  });

  const outroPaciente = paciente !== null && busca.paciente !== undefined && paciente.id !== busca.paciente;

  function autorizar(): void {
    setTentou(true);
    if (paciente === null || pontos === null || pontos === 0 || motivo === '' || busca.solicitante === undefined) return;
    op.enviar({ paciente_id: paciente.id, pontos_base: pontos, motivo, solicitante_id: busca.solicitante });
  }

  return (
    <>
      <CabecalhoDaTela titulo="Autorizar desconto" complemento={<span className="t14 mute">Acima do limite do caixa, com a sua senha e segundo fator</span>} />
      <section className="card pane fill" aria-label="Autorização de desconto">
        <div className="scroll col" style={{ gap: 14, maxWidth: 640 }}>
          {busca.solicitante === undefined && (
            <Aviso nivel="bloqueante" titulo="Pedido do caixa não identificado.">
              Abra esta tela pelo endereço que o caixa enviou no chat da equipe: ele leva o pedido de quem vai receber.
            </Aviso>
          )}
          {op.resultado !== null ? (
            <Aviso nivel="feito" titulo="Desconto autorizado.">
              Código da autorização <b className="num">{op.resultado.id}</b>, válido até {formatarDataHora(op.resultado.valida_ate, FUSO_DA_CLINICA)}.
              Envie o código ao caixa: o total muda quando ele registrar de novo. O lastro grava quem pediu, quem autorizou e o motivo.
            </Aviso>
          ) : (
            <form
              className="col"
              style={{ gap: 12 }}
              noValidate
              onSubmit={(ev) => {
                ev.preventDefault();
                autorizar();
              }}
            >
              {retomada?.retomar === 'autorizar' && op.erro === null && (
                <Aviso nivel="informativo" titulo="Identidade confirmada.">
                  Confirme de novo o paciente e o motivo e autorize.
                </Aviso>
              )}
              <h2 className="t15 b">Paciente</h2>
              {paciente === null ? (
                <>
                  <Busca rotulo="Buscar paciente por nome" valor={texto} aoMudar={setTexto} placeholder="Nome do paciente (mínimo 3 letras)" />
                  {tentou && <span className="campo-erro">Escolha o paciente pelo nome e nascimento.</span>}
                  {texto.trim().length >= 3 && (
                    <div>
                      <EstadoDaConsulta oQue="os pacientes" carregando={pacientes.isPending} erro={pacientes.error} aoTentarDeNovo={() => void pacientes.refetch()} linhas={2} />
                      {pacientes.data?.map((p) => (
                        <LinhaDoPaciente key={p.id} nome={p.nome} nascimento={p.nascimento} selecionada={false} aoAbrir={() => { setPaciente(p); }} />
                      ))}
                    </div>
                  )}
                </>
              ) : (
                <div className="row" style={{ gap: 10 }}>
                  <span style={{ flex: 1 }}>
                    <b>{paciente.nome}</b>, nascimento <b className="num">{formatarData(paciente.nascimento)}</b>
                  </span>
                  <Botao pequeno onPress={() => { setPaciente(null); }}>
                    Trocar paciente
                  </Botao>
                </div>
              )}
              {outroPaciente && (
                <Aviso nivel="atencao" titulo="Este não é o paciente do pedido do caixa.">
                  Confira o nome e o nascimento com o caixa antes de autorizar.
                </Aviso>
              )}
              <div className="grid g2" style={{ gap: 10 }}>
                <Campo
                  rotulo="Desconto autorizado (%)"
                  value={percentual}
                  onChange={setPercentual}
                  inputMode="decimal"
                  erro={tentou && (pontos === null || pontos === 0) ? 'Digite o percentual, ex.: 15.' : null}
                />
                <Escolha rotulo="Motivo" valor={motivo} aoMudar={setMotivo} vazio="Escolha o motivo" opcoes={MOTIVOS} erro={tentou && motivo === '' ? 'Escolha o motivo.' : null} />
              </div>
              {op.erro !== null && (
                <ErroDoAto
                  erro={op.erro}
                  ato="autorizar o desconto"
                  naoPreservado="o paciente escolhido e o motivo"
                  aoReautenticar={() => {
                    reautenticar('autorizar', op.chave());
                  }}
                  aoTentarDeNovo={autorizar}
                />
              )}
              <div className="row">
                <Botao type="submit" variante="principal" ocupado={op.enviando} isDisabled={busca.solicitante === undefined}>
                  Autorizar desconto
                </Botao>
              </div>
            </form>
          )}
        </div>
      </section>
    </>
  );
}
