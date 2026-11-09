import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ErroDaApi } from '@vidacor/api-client';
import type { CertificadoDoMedico, MedicoDoCorpoClinico, NovaConsultaCrm, NovoCertificado, SituacaoNoConselho } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Escolha, formatarDataHora } from '@vidacor/design-system';
import { useId, useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { useReautenticar } from '../../reautenticacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { CHAVE_CORPO_CLINICO } from './comum';

export const SITUACAO_NO_CONSELHO: Record<SituacaoNoConselho, string> = {
  regular: 'Regular',
  suspenso: 'Suspenso',
  cassado: 'Cassado',
  cancelado: 'Cancelado',
  falecido: 'Falecido',
  transferido: 'Transferido',
  interdicao_cautelar: 'Interdição cautelar',
  nao_encontrado: 'Não encontrado',
};

const DIA = new Intl.DateTimeFormat('pt-BR', { timeZone: FUSO_DA_CLINICA, day: '2-digit', month: '2-digit', year: 'numeric' });

/** Instante do servidor como dia de calendário na clínica ("06/11/2026"). */
function dia(instante: string): string {
  return DIA.format(new Date(instante));
}

const OPCOES_DA_SITUACAO = (Object.keys(SITUACAO_NO_CONSELHO) as SituacaoNoConselho[]).map((s) => ({ valor: s, rotulo: SITUACAO_NO_CONSELHO[s] }));

/** Aviso do certificado: vencido (bloqueante) ou vencendo em até 30 dias (atenção). */
export function AvisoDoCertificado({ c }: { c: CertificadoDoMedico | null | undefined }) {
  if (c === null || c === undefined) {
    return (
      <Aviso nivel="informativo" titulo="Certificado ICP-Brasil não cadastrado.">
        Cadastre o certificado público do médico para acompanhar o vencimento.
      </Aviso>
    );
  }
  if (c.vence_em_dias < 0) {
    return (
      <Aviso nivel="bloqueante" titulo={`Certificado vencido em ${dia(c.valido_ate)}.`}>
        O médico não consegue assinar laudos com este certificado. Peça a renovação e cadastre o novo.
      </Aviso>
    );
  }
  if (c.vence_em_dias <= 30) {
    return (
      <Aviso nivel="atencao" titulo={`Certificado vence em ${String(c.vence_em_dias)} dias (${dia(c.valido_ate)}).`}>
        Peça a renovação ao médico antes do vencimento para a assinatura não parar.
      </Aviso>
    );
  }
  return null;
}

function ConsultaNoCfm({ m }: { m: MedicoDoCorpoClinico }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const pendente = m.pendentes.find((p) => p.tipo === 'registro')?.registro ?? null;
  const uf = pendente?.uf ?? m.registro?.uf ?? '';
  const numero = pendente?.numero ?? m.registro?.numero ?? '';
  const [situacao, setSituacao] = useState('');
  const [observacao, setObservacao] = useState('');
  const [peloPortal, setPeloPortal] = useState(false);
  const [tentou, setTentou] = useState(false);
  const consultas = useQuery({ queryKey: [...CHAVE_CORPO_CLINICO, m.id, 'consultas-crm'], queryFn: () => api.corpoClinico.consultasCrm(m.id) });
  const op = useOperacao((n: NovaConsultaCrm, chave: string) => api.corpoClinico.consultarCrm(m.id, n, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      setPeloPortal(false);
      await cliente.invalidateQueries({ queryKey: CHAVE_CORPO_CLINICO });
    },
    aoFalhar: (e) => {
      if (e instanceof ErroDaApi && e.codigo === 'cfm-nao-configurado') setPeloPortal(true);
    },
  });
  if (uf === '' || numero === '') {
    return <p className="t14 mute">Peça primeiro o registro do CRM; a consulta ao CFM usa a UF e o número pedidos.</p>;
  }
  return (
    <section aria-label="Consulta ao CFM" className="col" style={{ gap: 10 }}>
      <p className="t14" style={{ margin: 0 }}>
        CRM-{uf} <span className="num">{numero}</span>. A aprovação do registro exige consulta regular dos últimos 30 dias.
      </p>
      <div className="row" style={{ gap: 8 }}>
        <Botao
          pequeno
          variante="principal"
          ocupado={op.enviando}
          onPress={() => {
            op.enviar({ uf, numero, fonte: 'webservice_cfm' });
          }}
        >
          Consultar no CFM
        </Botao>
        <Botao
          pequeno
          onPress={() => {
            setPeloPortal(true);
          }}
        >
          Registrar consulta feita no portal
        </Botao>
      </div>
      {op.erro !== null && !(op.erro instanceof ErroDaApi && op.erro.codigo === 'cfm-nao-configurado' && peloPortal) && <ErroDoAto erro={op.erro} ato="consultar o CRM" />}
      {peloPortal && (
        <form
          className="col"
          style={{ gap: 8 }}
          aria-label="Consulta feita no portal do CFM"
          noValidate
          onSubmit={(ev) => {
            ev.preventDefault();
            setTentou(true);
            if (situacao !== '' && observacao.trim().length >= 5)
              op.enviar({ uf, numero, fonte: 'portal_cfm', situacao: situacao as SituacaoNoConselho, observacao: observacao.trim() });
          }}
        >
          <Escolha rotulo="Situação no portal do CFM" valor={situacao} aoMudar={setSituacao} vazio="Escolha" opcoes={OPCOES_DA_SITUACAO} erro={tentou && situacao === '' ? 'Escolha a situação que o portal mostrou.' : null} />
          <Campo
            rotulo="Onde e como consultou"
            value={observacao}
            onChange={setObservacao}
            erro={tentou && observacao.trim().length < 5 ? 'Diga onde e como consultou (5 letras ou mais).' : null}
          />
          <div className="row">
            <Botao type="submit" pequeno variante="principal" ocupado={op.enviando}>
              Registrar consulta
            </Botao>
          </div>
        </form>
      )}
      <EstadoDaConsulta oQue="as consultas ao CFM" carregando={consultas.isPending} erro={consultas.error} aoTentarDeNovo={() => void consultas.refetch()} />
      <ol aria-label="Consultas ao CFM" style={{ listStyle: 'none', margin: 0, padding: 0 }}>
        {(consultas.data ?? []).map((c) => (
          <li key={c.id} className="t13" style={{ padding: '6px 0', borderBottom: '1px solid var(--linha)', overflowWrap: 'anywhere' }}>
            <b>{SITUACAO_NO_CONSELHO[c.situacao]}</b>, CRM-{c.uf} <span className="num">{c.numero}</span>, {c.fonte === 'webservice_cfm' ? 'consulta automática ao CFM' : 'portal do CFM'},{' '}
            {formatarDataHora(c.consultado_em, FUSO_DA_CLINICA)}, {c.consultado_por}.{c.observacao != null && ` ${c.observacao}.`}
          </li>
        ))}
      </ol>
    </section>
  );
}

function lerArquivo(arquivo: File): Promise<string> {
  return new Promise((resolver, rejeitar) => {
    const leitor = new FileReader();
    leitor.onload = () => {
      const r = typeof leitor.result === 'string' ? leitor.result : '';
      resolver(r.slice(r.indexOf(',') + 1));
    };
    leitor.onerror = () => {
      rejeitar(new Error('não foi possível ler o arquivo'));
    };
    leitor.readAsDataURL(arquivo);
  });
}

function CertificadoIcp({ m }: { m: MedicoDoCorpoClinico }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const reautenticar = useReautenticar();
  const idDoArquivo = useId();
  const [arquivo, setArquivo] = useState<File | null>(null);
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const c = m.certificado ?? null;
  const op = useOperacao((n: NovoCertificado, chave: string) => api.corpoClinico.cadastrarCertificado(m.id, n, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      setArquivo(null);
      setMotivo('');
      await cliente.invalidateQueries({ queryKey: CHAVE_CORPO_CLINICO });
    },
  });
  const precisaMotivo = c !== null;
  const enviar = async () => {
    setTentou(true);
    if (arquivo === null || (precisaMotivo && motivo.trim().length < 5)) return;
    const base64 = await lerArquivo(arquivo);
    op.enviar({ arquivo_base64: base64, motivo: motivo.trim() === '' ? null : motivo.trim() });
  };
  return (
    <section aria-label="Certificado ICP-Brasil" className="col" style={{ gap: 10 }}>
      {c !== null && (
        <dl className="kv" style={{ margin: 0, gridTemplateColumns: '120px minmax(0, 1fr)' }}>
          <dt>Titular</dt>
          <dd>
            {c.titular}
            {c.cpf_mascarado != null && `, CPF ${c.cpf_mascarado}`}
          </dd>
          <dt>Tipo e emissor</dt>
          <dd style={{ overflowWrap: 'anywhere' }}>
            {c.tipo ?? 'não informado'}, {c.emissor}
          </dd>
          <dt>Validade</dt>
          <dd>
            {dia(c.valido_de)} a {dia(c.valido_ate)}
          </dd>
          <dt>Impressão digital</dt>
          <dd className="num t13" style={{ overflowWrap: 'anywhere' }}>
            SHA-256 {c.sha256}
          </dd>
        </dl>
      )}
      <form
        className="col"
        style={{ gap: 8 }}
        aria-label={c === null ? 'Cadastrar certificado' : 'Substituir certificado'}
        noValidate
        onSubmit={(ev) => {
          ev.preventDefault();
          void enviar();
        }}
      >
        <div className="field">
          <label htmlFor={idDoArquivo}>Arquivo do certificado público (.cer, .crt ou .pem)</label>
          <input
            id={idDoArquivo}
            type="file"
            accept=".cer,.crt,.pem,.der"
            onChange={(ev) => {
              setArquivo(ev.target.files?.[0] ?? null);
            }}
          />
          {tentou && arquivo === null && <span className="campo-erro">Escolha o arquivo do certificado.</span>}
        </div>
        <p className="t13 mute" style={{ margin: 0 }}>
          Nunca envie o arquivo .pfx ou .p12 nem a senha: o núcleo só guarda os dados do certificado público.
        </p>
        {precisaMotivo && <Campo rotulo="Motivo da troca" value={motivo} onChange={setMotivo} erro={tentou && motivo.trim().length < 5 ? 'Diga o motivo (5 letras ou mais).' : null} />}
        {op.erro !== null && (
          <ErroDoAto
            erro={op.erro}
            ato="cadastrar o certificado"
            aoReautenticar={() => {
              reautenticar('certificado-corpo-clinico', op.chave(), {}, () => {
                void enviar();
              });
            }}
          />
        )}
        <div className="row">
          <Botao type="submit" pequeno variante="principal" ocupado={op.enviando}>
            {c === null ? 'Cadastrar certificado' : 'Substituir certificado'}
          </Botao>
        </div>
      </form>
    </section>
  );
}

/** Aba "CFM e certificado": consulta do CRM e certificado ICP-Brasil (DEC-2026-11-09-120 e 113). */
export function ConferenciasDoMedico({ m }: { m: MedicoDoCorpoClinico }) {
  return (
    <div className="col" style={{ gap: 16 }}>
      <ConsultaNoCfm m={m} />
      <CertificadoIcp m={m} />
    </div>
  );
}
