import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { Glosa } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, Chip, formatarData, formatarDataHora, formatarDinheiro } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA } from '../agenda/grade';
import { chavesDoFaturamento } from './DetalheDaConta';
import { SITUACAO_DA_CONTA, SITUACAO_DA_GLOSA, SITUACAO_DO_LOTE } from './rotulos';

/** Lote TISS: guias, hashes, envio (pendente fica visível, R7) e o arquivo para conferência. */
export function DetalheDoLote({ id, podeEnviar }: { id: string; podeEnviar: boolean }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const consulta = useQuery({ queryKey: chavesDoFaturamento.lote(id), queryFn: () => api.faturamento.lote(id) });
  const [erro, setErro] = useState<unknown>(null);
  const [enviando, setEnviando] = useState(false);
  if (consulta.data === undefined) {
    return <EstadoDaConsulta oQue="o lote" carregando={consulta.isPending} erro={consulta.error} aoTentarDeNovo={() => void consulta.refetch()} />;
  }
  const { lote, contas } = consulta.data;
  const s = SITUACAO_DO_LOTE[lote.situacao];
  return (
    <>
      <div className="row wrap" style={{ gap: 10 }}>
        <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0, flex: 1 }}>
          Lote {lote.numero}, {lote.operadora}
        </h2>
        <Chip cor={s.cor} texto={s.texto} />
      </div>
      <p className="t14" style={{ margin: 0 }}>
        {lote.codigo}, {lote.quantidade_guias} guia(s), {formatarDinheiro(lote.valor_total_centavos)}. TISS {lote.versao_tiss}, gerado em{' '}
        {formatarDataHora(lote.criado_em, FUSO_DA_CLINICA)}.
      </p>
      <p className="t13 mute" style={{ margin: 0, overflowWrap: 'anywhere' }}>
        Hash do epílogo {lote.hash_tiss}. SHA-256 do arquivo {lote.sha256}.
      </p>
      {lote.protocolo != null && (
        <p className="t14" style={{ margin: 0 }}>
          Protocolo da operadora <b className="num">{lote.protocolo}</b>
          {lote.enviado_em != null && `, enviado em ${formatarDataHora(lote.enviado_em, FUSO_DA_CLINICA)}`}.
        </p>
      )}
      {lote.envio_pendente != null && (
        <Aviso nivel="atencao" titulo="Envio pendente: a operadora ainda não recebeu este lote.">
          {`O servidor tenta de novo sozinho (tentativa ${String(lote.envio_pendente.tentativas)}). Último erro: ${lote.envio_pendente.ultimo_erro ?? 'sem detalhe'}. Nada se perdeu.`}
        </Aviso>
      )}
      {lote.valor_pago_centavos != null && (
        <p className="t14" style={{ margin: 0 }}>
          Retorno: pago {formatarDinheiro(lote.valor_pago_centavos)}, glosado {formatarDinheiro(lote.valor_glosado_centavos ?? 0)}.
        </p>
      )}
      <div className="row wrap" style={{ gap: 10 }}>
        {podeEnviar && lote.situacao === 'gerado' && (
          <Botao
            variante="principal"
            ocupado={enviando}
            onPress={() => {
              setErro(null);
              setEnviando(true);
              api.faturamento
                .enviarLote(lote.id)
                .then(async (d) => {
                  cliente.setQueryData(chavesDoFaturamento.lote(lote.id), d);
                  await cliente.invalidateQueries({ queryKey: chavesDoFaturamento.tudo });
                }, setErro)
                .finally(() => {
                  setEnviando(false);
                });
            }}
          >
            Enviar à operadora
          </Botao>
        )}
        {podeEnviar && (
          <a className="btn" href={api.faturamento.enderecoDoArquivo(lote.id)} download={`${lote.codigo}.xml`}>
            Baixar o XML do lote
          </a>
        )}
      </div>
      {erro !== null && <ErroDoAto erro={erro} ato="enviar o lote" />}
      <section aria-label="Guias do lote" className="col" style={{ gap: 4 }}>
        {contas.map((c) => (
          <div key={c.id} className="row t14" style={{ gap: 10 }}>
            <span style={{ flex: 1, minWidth: 0 }}>
              {c.paciente.nome}, <span className="num">{formatarData(c.paciente.nascimento)}</span>, {c.codigo}
            </span>
            <span className="num">{formatarDinheiro(c.valor_total_centavos)}</span>
            <Chip cor={SITUACAO_DA_CONTA[c.situacao].cor} texto={SITUACAO_DA_CONTA[c.situacao].texto} />
          </div>
        ))}
      </section>
    </>
  );
}

/** Glosa recebida e recurso com justificativa (Faturamento.dc.html, "Recurso da guia"). */
export function DetalheDaGlosa({ glosa, podeRecorrer }: { glosa: Glosa; podeRecorrer: boolean }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [texto, setTexto] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((_: undefined, chave: string) => api.faturamento.recorrer(glosa.id, { justificativa: texto.trim() }, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: chavesDoFaturamento.tudo }),
  });
  const tamanho = texto.trim().length;
  const ok = tamanho >= 10 && tamanho <= 500;
  const s = SITUACAO_DA_GLOSA[glosa.situacao];
  return (
    <>
      <div className="row wrap" style={{ gap: 10 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h2 style={{ fontSize: 20, fontWeight: 650, margin: 0 }}>Recurso da guia {glosa.conta_codigo}</h2>
          <div className="t14 mute">
            {glosa.paciente.nome}, nascimento <span className="num">{formatarData(glosa.paciente.nascimento)}</span>
          </div>
        </div>
        <Chip cor={s.cor} texto={s.texto} />
      </div>
      <p className="t14" style={{ margin: 0 }}>
        Glosa {glosa.codigo_glosa}, {glosa.motivo}. Valor {formatarDinheiro(glosa.valor_centavos)}. Recurso até {formatarData(glosa.prazo_recurso)}.
      </p>
      {glosa.recurso != null && (
        <Aviso nivel="informativo" titulo={`Recurso ${glosa.recurso.codigo} registrado.`}>
          {`${glosa.recurso.justificativa} O envio à operadora sai pelo servidor e fica visível até ser recebido.`}
        </Aviso>
      )}
      {podeRecorrer && glosa.situacao === 'aberta' && (
        <form
          className="col"
          style={{ gap: 10 }}
          aria-label="Recurso da glosa"
          noValidate
          onSubmit={(ev) => {
            ev.preventDefault();
            setTentou(true);
            if (ok) op.enviar(undefined);
          }}
        >
          <AreaDeTexto
            rotulo="Justificativa"
            value={texto}
            onChange={setTexto}
            linhas={4}
            ajuda="De 10 a 500 caracteres, o limite do TISS."
            erro={tentou && !ok ? 'Escreva a justificativa (de 10 a 500 caracteres).' : null}
          />
          {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar o recurso" />}
          <div className="row">
            <Botao type="submit" variante="principal" ocupado={op.enviando}>
              Registrar recurso
            </Botao>
          </div>
        </form>
      )}
    </>
  );
}
