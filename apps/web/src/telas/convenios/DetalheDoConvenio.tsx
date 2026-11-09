import type { ConvenioDetalhe, MudancaDoConvenio } from '@vidacor/api-client';
import { Abas, Botao, Chip, formatarDataHora, ListaDeDados, Tabela } from '@vidacor/design-system';
import { useState } from 'react';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { dataCurta, EFEITO, SITUACAO } from './comum';
import { RegrasDoConvenio } from './RegrasDoConvenio';
import { TabelaDoConvenio } from './TabelaDoConvenio';

const TIPO: Record<MudancaDoConvenio['tipo'], string> = {
  cadastrar: 'Cadastrado',
  alterar: 'Alterado',
  desativar: 'Desativado',
  reativar: 'Reativado',
};

function ou(v: string | number | null | undefined, vazio = 'Não informado'): string {
  return v === null || v === undefined || v === '' ? vazio : String(v);
}

function Contrato({ c, faturista, aoMudar }: { c: ConvenioDetalhe; faturista: boolean; aoMudar: () => void }) {
  const t = c.termos;
  return (
    <div className="col" style={{ gap: 12 }}>
      <ListaDeDados
        larguraDoRotulo={190}
        itens={[
          { rotulo: 'Vigência', valor: t.vigencia_inicio === null || t.vigencia_inicio === undefined ? 'Não informada' : `${dataCurta(t.vigencia_inicio)} a ${typeof t.vigencia_fim === 'string' ? dataCurta(t.vigencia_fim) : 'sem fim'}` },
          { rotulo: 'Reajuste', valor: ou(t.reajuste) },
          { rotulo: 'Envio de lote', valor: `Até ${String(t.prazo_envio_dias)} dias após o atendimento` },
          { rotulo: 'Pagamento', valor: typeof t.prazo_pagamento_dias === 'number' ? `${String(t.prazo_pagamento_dias)} dias após o envio` : 'Não informado' },
          { rotulo: 'Recurso de glosa', valor: `Até ${String(t.prazo_recurso_dias)} dias do demonstrativo` },
          { rotulo: 'Prestador na operadora', valor: `${t.nome_prestador}, código ${t.codigo_prestador}, CNES ${t.cnes}` },
          { rotulo: 'Webservice TISS', valor: t.autorizacao_por_webservice ? 'Autorização pelo webservice' : 'Autorização pelo portal da operadora' },
          { rotulo: 'Contato', valor: [t.contato_nome, t.contato_telefone, t.contato_email].filter((x) => typeof x === 'string' && x !== '').join(', ') || 'Não informado' },
        ]}
      />
      {faturista && (
        <div className="row">
          <Botao pequeno onPress={aoMudar}>
            Mudar contrato
          </Botao>
        </div>
      )}
    </div>
  );
}

function Auditoria({ c }: { c: ConvenioDetalhe }) {
  return (
    <div className="col" style={{ gap: 10 }}>
      <Tabela
        rotulo="Conferências antes do lote"
        colunas={[
          {
            id: 'regra',
            rotulo: 'Conferência antes do lote',
            cabecalhoDaLinha: true,
            celula: (r) => (
              <>
                <div className="b">{r.titulo}</div>
                <div className="t13 mute">{r.origem}</div>
              </>
            ),
          },
          { id: 'efeito', rotulo: 'Efeito', celula: (r) => <Chip cor={EFEITO[r.efeito].cor} texto={EFEITO[r.efeito].texto} /> },
        ]}
        linhas={c.auditoria.regras.map((r) => ({ ...r, id: r.regra }))}
      />
      <div className="t13 mute">
        Conjunto {c.auditoria.nome} ({c.auditoria.versao}). Trocar o conjunto é mudança do contrato, com motivo.
      </div>
    </div>
  );
}

function Historico({ c }: { c: ConvenioDetalhe }) {
  return (
    <ul className="col" style={{ gap: 8, listStyle: 'none', margin: 0, padding: 0 }}>
      {c.historico.map((m) => (
        <li key={m.codigo} style={{ borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
          <div className="row" style={{ gap: 8 }}>
            <b style={{ flex: 1, minWidth: 0 }}>
              {TIPO[m.tipo]}, versão {m.versao_do_contrato}
            </b>
            <span className="t13 mute">{formatarDataHora(m.em, FUSO_DA_CLINICA)}</span>
          </div>
          <div className="t13 mute">
            {m.quem}: {m.motivo}.
          </div>
        </li>
      ))}
      {c.tabelas.map((t) => (
        <li key={t.id} style={{ borderBottom: '1px solid var(--line)', paddingBottom: 8 }}>
          <div className="row" style={{ gap: 8 }}>
            <b style={{ flex: 1, minWidth: 0 }}>
              Tabela v{t.versao} {t.descricao}, desde {dataCurta(t.vigente_desde)}
            </b>
            <span className="t13 mute">{formatarDataHora(t.criada_em, FUSO_DA_CLINICA)}</span>
          </div>
          <div className="t13 mute">
            {typeof t.proposta_por === 'string' ? `Proposta por ${t.proposta_por}, aprovada por ${ou(t.aprovada_por, 'gerência')}` : 'Carga inicial'}
            {typeof t.motivo === 'string' ? `: ${t.motivo}` : ''}. {t.itens} preços.
          </div>
        </li>
      ))}
    </ul>
  );
}

export function DetalheDoConvenio({ c, faturista, gerencia, eu, aoMudarContrato }: { c: ConvenioDetalhe; faturista: boolean; gerencia: boolean; eu: string; aoMudarContrato: () => void }) {
  const [aba, setAba] = useState('tabela');
  const s = SITUACAO[c.situacao];
  return (
    <div className="col" style={{ gap: 10 }}>
      <div className="row wrap" style={{ gap: 12 }}>
        <div style={{ flex: '1 1 260px', minWidth: 0 }}>
          <h2 style={{ margin: 0, fontSize: 20, fontWeight: 650 }}>{c.operadora}</h2>
          <div className="t13 mute num">
            ANS {c.registro_ans}, contrato {c.codigo}
          </div>
        </div>
        <Chip cor={s.cor} texto={typeof c.termos.vigencia_fim === 'string' && c.situacao !== 'inativo' ? `${s.texto}, até ${dataCurta(c.termos.vigencia_fim)}` : s.texto} />
      </div>
      <Abas
        rotulo="Seções do convênio"
        selecionada={aba}
        aoSelecionar={setAba}
        abas={[
          { id: 'contrato', rotulo: 'Contrato', conteudo: <Contrato c={c} faturista={faturista} aoMudar={aoMudarContrato} /> },
          { id: 'tabela', rotulo: 'Tabela de preços', conteudo: <TabelaDoConvenio key={c.id} c={c} faturista={faturista} gerencia={gerencia} eu={eu} /> },
          { id: 'autorizacao', rotulo: 'Autorização', conteudo: <RegrasDoConvenio key={c.id} c={c} faturista={faturista} /> },
          { id: 'auditoria', rotulo: 'Auditoria', conteudo: <Auditoria c={c} /> },
          { id: 'historico', rotulo: 'Histórico', conteudo: <Historico c={c} /> },
        ]}
      />
    </div>
  );
}
