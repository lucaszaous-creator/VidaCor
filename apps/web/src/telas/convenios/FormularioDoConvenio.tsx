import { parseDate, type CalendarDate } from '@internationalized/date';
import { useQueryClient } from '@tanstack/react-query';
import type { ConjuntoDeAuditoriaDisponivel, ConvenioDetalhe, TermosDoConvenio } from '@vidacor/api-client';
import { Aviso, Botao, Campo, CampoDeData, CaixaDeMarcar, Escolha } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { CHAVE_CONVENIOS } from './comum';

function data(iso: string | null | undefined): CalendarDate | null {
  return typeof iso === 'string' ? parseDate(iso) : null;
}

function numero(t: string): number | null {
  return /^\d{1,3}$/.test(t.trim()) ? Number(t.trim()) : null;
}

const PRAZOS = (t: string) => {
  const n = numero(t);
  return n !== null && n >= 1 && n <= 365;
};

/**
 * Cadastro (sem `atual`) ou mudança dos termos do convênio (faturamento). O formulário manda o estado
 * inteiro com a versão vista e o motivo; quem confere cada campo é o servidor.
 */
export function FormularioDoConvenio({
  atual,
  disponiveis,
  aoConcluir,
  aoFechar,
}: {
  atual: ConvenioDetalhe | null;
  disponiveis: ConjuntoDeAuditoriaDisponivel[];
  aoConcluir: (id: string) => void;
  aoFechar: () => void;
}) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const t = atual?.termos;
  const [operadora, setOperadora] = useState('');
  const [ans, setAns] = useState('');
  const [codigo, setCodigo] = useState(t?.codigo_prestador ?? '');
  const [nome, setNome] = useState(t?.nome_prestador ?? '');
  const [cnes, setCnes] = useState(t?.cnes ?? '');
  const [regra, setRegra] = useState(t?.regra_versao ?? '');
  const [envio, setEnvio] = useState(t === undefined ? '30' : String(t.prazo_envio_dias));
  const [recurso, setRecurso] = useState(t === undefined ? '30' : String(t.prazo_recurso_dias));
  const [pagamento, setPagamento] = useState(typeof t?.prazo_pagamento_dias === 'number' ? String(t.prazo_pagamento_dias) : '');
  const [inicio, setInicio] = useState<CalendarDate | null>(data(t?.vigencia_inicio));
  const [fim, setFim] = useState<CalendarDate | null>(data(t?.vigencia_fim));
  const [reajuste, setReajuste] = useState(t?.reajuste ?? '');
  const [contato, setContato] = useState(t?.contato_nome ?? '');
  const [telefone, setTelefone] = useState(t?.contato_telefone ?? '');
  const [email, setEmail] = useState(t?.contato_email ?? '');
  const [webservice, setWebservice] = useState(t?.autorizacao_por_webservice ?? false);
  const [ativo, setAtivo] = useState(t?.ativo ?? true);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao(
    (termos: TermosDoConvenio, chave: string) =>
      atual === null
        ? api.convenios.cadastrar({ operadora: operadora.trim(), registro_ans: ans.trim(), termos, motivo: motivo.trim() }, { idempotencyKey: chave })
        : api.convenios.alterar(atual.id, { versao: atual.versao, termos, motivo: motivo.trim() }, { idempotencyKey: chave }),
    {
      aoConcluir: async (c) => {
        await cliente.invalidateQueries({ queryKey: CHAVE_CONVENIOS });
        aoConcluir(c.id);
      },
    },
  );
  const erros = {
    operadora: atual === null && operadora.trim().length < 2 ? 'Diga o nome da operadora como vem na guia.' : null,
    ans: atual === null && !/^\d{6}$/.test(ans.replace(/[.-]/g, '').trim()) ? 'O registro na ANS tem 6 algarismos.' : null,
    codigo: /^[A-Za-z0-9]{1,14}$/.test(codigo.trim()) ? null : 'Até 14 letras ou algarismos.',
    nome: nome.trim().length >= 1 ? null : 'Diga o nome do prestador na operadora.',
    cnes: /^\d{7}$/.test(cnes.trim()) ? null : 'O CNES tem 7 algarismos.',
    regra: regra === '' ? 'Escolha o conjunto de regras.' : null,
    envio: PRAZOS(envio) ? null : 'De 1 a 365 dias.',
    recurso: PRAZOS(recurso) ? null : 'De 1 a 365 dias.',
    pagamento: pagamento.trim() === '' || PRAZOS(pagamento) ? null : 'De 1 a 365 dias.',
    motivo: motivo.trim().length >= 5 ? null : 'Diga o motivo (5 letras ou mais).',
  };
  const ok = Object.values(erros).every((e) => e === null);
  const titulo = atual === null ? 'Cadastrar operadora' : `Mudar contrato, ${atual.operadora}`;
  if (op.resultado !== null) {
    return (
      <Aviso
        nivel="feito"
        titulo={atual === null ? `${op.resultado.operadora} cadastrada.` : `Contrato de ${op.resultado.operadora}: versão ${String(op.resultado.versao)} registrada.`}
        acoes={
          <Botao pequeno onPress={aoFechar}>
            Fechar
          </Botao>
        }
      >
        A mudança ficou no histórico do convênio, com o motivo, e no lastro.
      </Aviso>
    );
  }
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label={titulo}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (ok)
          op.enviar({
            codigo_prestador: codigo.trim(),
            nome_prestador: nome.trim(),
            cnes: cnes.trim(),
            regra_versao: regra,
            prazo_envio_dias: Number(envio),
            prazo_recurso_dias: Number(recurso),
            prazo_pagamento_dias: pagamento.trim() === '' ? null : Number(pagamento),
            vigencia_inicio: inicio?.toString() ?? null,
            vigencia_fim: fim?.toString() ?? null,
            reajuste: reajuste.trim() === '' ? null : reajuste.trim(),
            contato_nome: contato.trim() === '' ? null : contato.trim(),
            contato_telefone: telefone.trim() === '' ? null : telefone.trim(),
            contato_email: email.trim() === '' ? null : email.trim(),
            autorizacao_por_webservice: webservice,
            ativo,
          });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        {titulo}
      </h2>
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        {atual === null && (
          <>
            <Campo rotulo="Operadora" value={operadora} onChange={setOperadora} erro={tentou ? erros.operadora : null} />
            <Campo rotulo="Registro na ANS" value={ans} onChange={setAns} inputMode="numeric" erro={tentou ? erros.ans : null} />
          </>
        )}
        <Campo rotulo="Código do prestador na operadora" value={codigo} onChange={setCodigo} erro={tentou ? erros.codigo : null} />
        <Campo rotulo="CNES" value={cnes} onChange={setCnes} inputMode="numeric" erro={tentou ? erros.cnes : null} />
        <Campo rotulo="Nome do prestador na operadora" value={nome} onChange={setNome} erro={tentou ? erros.nome : null} />
        <Escolha
          rotulo="Regras da auditoria"
          valor={regra}
          aoMudar={setRegra}
          vazio="Escolha o conjunto"
          opcoes={disponiveis.map((d) => ({ valor: d.versao, rotulo: `${d.nome} (${d.versao})` }))}
          erro={tentou ? erros.regra : null}
        />
        <CampoDeData rotulo="Vigência a partir de" valor={inicio} aoMudar={setInicio} />
        <CampoDeData rotulo="Vigência até" valor={fim} aoMudar={setFim} />
        <Campo rotulo="Prazo de envio do lote (dias)" value={envio} onChange={setEnvio} inputMode="numeric" erro={tentou ? erros.envio : null} />
        <Campo rotulo="Prazo de recurso de glosa (dias)" value={recurso} onChange={setRecurso} inputMode="numeric" erro={tentou ? erros.recurso : null} />
        <Campo rotulo="Pagamento após o envio (dias)" value={pagamento} onChange={setPagamento} inputMode="numeric" erro={tentou ? erros.pagamento : null} />
        <Campo rotulo="Reajuste" value={reajuste} onChange={setReajuste} placeholder="IPCA em janeiro" />
        <Campo rotulo="Contato na operadora" value={contato} onChange={setContato} />
        <Campo rotulo="Telefone do contato" value={telefone} onChange={setTelefone} inputMode="tel" />
        <Campo rotulo="E-mail do contato" value={email} onChange={setEmail} inputMode="email" />
      </div>
      <CaixaDeMarcar rotulo="Pedido de autorização pelo webservice TISS" marcado={webservice} aoMudar={setWebservice} />
      {atual !== null && (
        <CaixaDeMarcar
          rotulo="Convênio ativo"
          marcado={ativo}
          aoMudar={setAtivo}
          detalhe="Desativado, o convênio não recebe tabela nova; o histórico e as guias já feitas continuam."
        />
      )}
      <Campo rotulo="Motivo" value={motivo} onChange={setMotivo} erro={tentou ? erros.motivo : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato={atual === null ? 'cadastrar a operadora' : 'mudar o contrato'} />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          {atual === null ? 'Cadastrar' : 'Salvar mudança'}
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
    </form>
  );
}
