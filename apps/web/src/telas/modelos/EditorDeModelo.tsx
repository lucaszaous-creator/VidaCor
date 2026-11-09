import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { ModeloDeLaudo, NovaVersaoDeModelo, NovoModeloDeLaudo, VersaoDeModelo } from '@vidacor/api-client';
import { AreaDeTexto, Aviso, Botao, Campo, Chip, Escolha, formatarData, Tabela, type CorDoChip } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

export const CHAVE_MODELOS = ['modelos-de-laudo'] as const;

export const SITUACAO_DA_VERSAO: Record<VersaoDeModelo['situacao'], { texto: string; cor: CorDoChip }> = {
  proposta: { texto: 'Aguarda aprovação', cor: 'ambar' },
  publicada: { texto: 'Publicada', cor: 'verde' },
  substituida: { texto: 'Substituída', cor: 'cinza' },
  recusada: { texto: 'Recusada', cor: 'rosa' },
};

export const tipoDoModelo = (m: ModeloDeLaudo): { texto: string; cor: CorDoChip } =>
  m.tipo === 'clinica' ? { texto: 'Modelo da clínica', cor: 'marinho' } : { texto: 'Modelo pessoal', cor: 'violeta' };

/** Decisão da coordenação médica sobre a proposta (quem propôs não decide; o servidor confere). */
function DecisaoDaProposta({ m, proposta }: { m: ModeloDeLaudo; proposta: VersaoDeModelo }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [motivo, setMotivo] = useState('');
  const [tentouRecusar, setTentouRecusar] = useState(false);
  const decidir = useMutation({
    mutationFn: (aprovar: boolean) => api.modelosDeLaudo.decidir(m.id, proposta.numero, { aprovar, motivo: motivo.trim() === '' ? null : motivo.trim() }),
    onSuccess: () => cliente.invalidateQueries({ queryKey: CHAVE_MODELOS }),
  });
  const motivoOk = motivo.trim().length >= 5;
  return (
    <div className="col" style={{ gap: 8 }}>
      <Campo
        rotulo="Motivo da decisão"
        value={motivo}
        onChange={setMotivo}
        ajuda="Obrigatório para recusar; vai para quem propôs."
        erro={tentouRecusar && !motivoOk ? 'Para recusar, escreva o motivo (5 letras ou mais).' : null}
      />
      {decidir.isError && <ErroDoAto erro={decidir.error} ato="decidir a proposta" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao variante="principal" ocupado={decidir.isPending} onPress={() => { decidir.mutate(true); }}>
          Aprovar versão {proposta.numero}
        </Botao>
        <Botao
          onPress={() => {
            setTentouRecusar(true);
            if (motivoOk) decidir.mutate(false);
          }}
        >
          Recusar
        </Botao>
      </div>
    </div>
  );
}

function Desativar({ m }: { m: ModeloDeLaudo }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const desativar = useMutation({
    mutationFn: () => api.modelosDeLaudo.desativar(m.id, motivo.trim()),
    onSuccess: () => cliente.invalidateQueries({ queryKey: CHAVE_MODELOS }),
  });
  const ok = motivo.trim().length >= 5;
  return (
    <details style={{ borderTop: '1px solid var(--line)', paddingTop: 8 }}>
      <summary className="t14" style={{ cursor: 'pointer', minHeight: 44, display: 'flex', alignItems: 'center' }}>
        Tirar o modelo de uso
      </summary>
      <div className="col" style={{ gap: 8 }}>
        <Campo rotulo="Motivo para desativar" value={motivo} onChange={setMotivo} ajuda="Laudos que já usaram o modelo continuam com a versão gravada." erro={tentou && !ok ? 'Escreva o motivo (5 letras ou mais).' : null} />
        {desativar.isError && <ErroDoAto erro={desativar.error} ato="desativar o modelo" />}
        <div className="row">
          <Botao
            ocupado={desativar.isPending}
            onPress={() => {
              setTentou(true);
              if (ok) desativar.mutate();
            }}
          >
            Desativar modelo
          </Botao>
        </div>
      </div>
    </details>
  );
}

/** Editor do modelo: nova versão com motivo (pessoal publica; da clínica vai para aprovação). */
export function EditorDeModelo({ m }: { m: ModeloDeLaudo }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const publicada = m.versoes.find((v) => v.situacao === 'publicada');
  const proposta = m.versoes.find((v) => v.situacao === 'proposta');
  const [texto, setTexto] = useState((proposta ?? publicada)?.texto ?? '');
  const [motivo, setMotivo] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((p: NovaVersaoDeModelo, chave: string) => api.modelosDeLaudo.novaVersao(m.id, p, { idempotencyKey: chave }), {
    aoConcluir: async () => {
      setMotivo('');
      setTentou(false);
      await cliente.invalidateQueries({ queryKey: CHAVE_MODELOS });
    },
  });
  /** Mudar o texto depois de gravar começa outra operação (chave nova, G3). */
  const mudarTexto = (t: string): void => {
    if (op.resultado !== null || op.erro !== null) op.reiniciar();
    setTexto(t);
  };
  const clinica = m.tipo === 'clinica';
  const mudou = texto.trim() !== (publicada?.texto ?? '').trim();
  const motivoOk = motivo.trim().length >= 5;
  const proximo = Math.max(0, ...m.versoes.map((v) => v.numero)) + 1;
  return (
    <div className="col" style={{ gap: 12 }}>
      {proposta !== undefined && (
        <Aviso nivel="atencao" titulo={`Versão ${String(proposta.numero)} aguarda a coordenação médica.`}>
          Proposta por {proposta.proposta_por}: {proposta.motivo ?? 'primeira versão'}. Até a decisão, todos seguem com a{' '}
          {publicada === undefined ? 'versão anterior' : `versão ${String(publicada.numero)}`}.
        </Aviso>
      )}
      {proposta !== undefined && m.pode_decidir && <DecisaoDaProposta m={m} proposta={proposta} />}
      {!m.ativo && <Aviso nivel="informativo" titulo="Modelo desativado.">{m.motivo_desativacao}</Aviso>}
      <AreaDeTexto
        rotulo={proposta === undefined ? `Texto do modelo, versão ${String(publicada?.numero ?? 1)}` : `Texto proposto, versão ${String(proposta.numero)}`}
        value={texto}
        onChange={mudarTexto}
        linhas={10}
        isReadOnly={!m.pode_editar}
        ajuda="Campos variáveis entre colchetes, como [medida], são preenchidos no laudo. O modelo não leva dado de paciente."
      />
      {m.pode_editar ? (
        <>
          <Campo rotulo="O que mudou e por quê" value={motivo} onChange={setMotivo} erro={tentou && !motivoOk ? 'Escreva o motivo (5 letras ou mais).' : null} />
          <div aria-live="polite">
            {op.resultado !== null && (
              <Aviso nivel="feito" titulo={clinica ? 'Enviado para aprovação.' : `Versão ${String(op.resultado.versao_publicada ?? proximo)} publicada.`}>
                {clinica
                  ? 'A coordenação médica decide; até lá, todos seguem com a versão publicada.'
                  : 'Vale para laudos iniciados a partir de agora; os anteriores mantêm a versão usada. Registrado no lastro.'}
              </Aviso>
            )}
            {op.erro !== null && <ErroDoAto erro={op.erro} ato="gravar a nova versão do modelo" />}
          </div>
          <div className="row" style={{ gap: 8 }}>
            <Botao
              variante="principal"
              ocupado={op.enviando}
              isDisabled={!mudou}
              onPress={() => {
                setTentou(true);
                if (motivoOk) op.enviar({ texto, motivo: motivo.trim(), versao_base: m.versao_publicada ?? null });
              }}
            >
              {clinica ? `Enviar versão ${String(proximo)} para aprovação` : `Publicar versão ${String(proximo)}`}
            </Botao>
            {!mudou && <span className="t13 mute">Mude o texto para criar uma versão nova.</span>}
          </div>
        </>
      ) : (
        m.ativo && (
          <p className="t13 mute" style={{ margin: 0 }}>
            {proposta === undefined ? 'Você só lê este modelo.' : 'Há uma proposta em aberto: aguarde a decisão para propor outra.'}
          </p>
        )
      )}
      {m.pode_desativar && <Desativar m={m} />}
    </div>
  );
}

export function VersoesDoModelo({ m }: { m: ModeloDeLaudo }) {
  return (
    <div className="col" style={{ gap: 8 }}>
      <Tabela
        rotulo={`Versões de ${m.nome}`}
        colunas={[
          { id: 'n', rotulo: 'Versão', semQuebra: true, celula: (v) => <b className="num">v{v.numero}</b> },
          { id: 's', rotulo: 'Situação', semQuebra: true, celula: (v) => <Chip cor={SITUACAO_DA_VERSAO[v.situacao].cor} texto={SITUACAO_DA_VERSAO[v.situacao].texto} /> },
          {
            id: 'm',
            rotulo: 'Mudança',
            celula: (v) => (
              <span>
                {v.motivo ?? 'Primeira versão'}
                <span className="t13 mute" style={{ display: 'block' }}>
                  Proposta por {v.proposta_por}
                  {v.decidido_por === null || v.decidido_por === undefined ? '' : `, decidida por ${v.decidido_por}`}
                  {v.motivo_decisao === null || v.motivo_decisao === undefined ? '' : `: ${v.motivo_decisao}`}
                </span>
              </span>
            ),
          },
          { id: 'd', rotulo: 'Data', semQuebra: true, celula: (v) => formatarData((v.decidido_em ?? v.criado_em).slice(0, 10)) },
        ]}
        linhas={m.versoes.map((v) => ({ ...v, id: String(v.numero) }))}
      />
      <p className="t13 mute" style={{ margin: 0 }}>
        Cada laudo guarda a versão do modelo usada. Laudos assinados nunca mudam.
      </p>
    </div>
  );
}

export function QuemPodeEditar() {
  const linhas = [
    { id: 'clinica', tipo: 'Da clínica', quem: 'Médicos que laudam o exame propõem mudança', publicar: 'Aprovação da coordenação médica. Quem propõe não aprova.' },
    { id: 'pessoal', tipo: 'Pessoal', quem: 'Só o próprio médico', publicar: 'Publica direto.' },
    { id: 'frases', tipo: 'Frases rápidas', quem: 'Cada médico as suas; as da clínica, a coordenação médica', publicar: 'Pessoais sem aprovação.' },
  ];
  return (
    <Tabela
      rotulo="Quem pode editar"
      colunas={[
        { id: 'tipo', rotulo: 'Tipo', cabecalhoDaLinha: true, semQuebra: true, celula: (l) => l.tipo },
        { id: 'quem', rotulo: 'Quem edita', celula: (l) => l.quem },
        { id: 'publicar', rotulo: 'Para publicar', celula: (l) => l.publicar },
      ]}
      linhas={linhas}
    />
  );
}

/** Novo modelo: pessoal (publica já) ou proposta de modelo da clínica. */
export function NovoModelo({ aoFechar, aoCriar }: { aoFechar: () => void; aoCriar: (id: string) => void }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [tipo, setTipo] = useState<'pessoal' | 'clinica'>('pessoal');
  const [nome, setNome] = useState('');
  const [exame, setExame] = useState('');
  const [modalidade, setModalidade] = useState('');
  const [texto, setTexto] = useState('');
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovoModeloDeLaudo, chave: string) => api.modelosDeLaudo.criar(n, { idempotencyKey: chave }), {
    aoConcluir: async (m) => {
      await cliente.invalidateQueries({ queryKey: CHAVE_MODELOS });
      aoCriar(m.id);
    },
  });
  const nomeOk = nome.trim().length >= 3;
  const exameOk = exame.trim().length >= 3;
  const modOk = /^[A-Za-z]{2,4}$/.test(modalidade.trim());
  const textoOk = texto.trim().length >= 1;
  return (
    <form
      className="col"
      style={{ gap: 10 }}
      aria-label="Novo modelo"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (nomeOk && exameOk && modOk && textoOk) op.enviar({ tipo, nome: nome.trim(), exame: exame.trim(), modalidade: modalidade.trim().toUpperCase(), texto });
      }}
    >
      <h2 className="t15 b" style={{ margin: 0 }}>
        Novo modelo
      </h2>
      <Escolha
        rotulo="Tipo"
        valor={tipo}
        aoMudar={(v) => {
          setTipo(v === 'clinica' ? 'clinica' : 'pessoal');
        }}
        opcoes={[
          { valor: 'pessoal', rotulo: 'Pessoal (só meu, publica já)' },
          { valor: 'clinica', rotulo: 'Da clínica (vai para aprovação)' },
        ]}
      />
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 2fr) minmax(0, 2fr) minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Campo rotulo="Nome do modelo" value={nome} onChange={setNome} erro={tentou && !nomeOk ? 'Dê um nome (3 letras ou mais).' : null} />
        <Campo rotulo="Exame" value={exame} onChange={setExame} erro={tentou && !exameOk ? 'Diga o exame.' : null} />
        <Campo rotulo="Modalidade" value={modalidade} onChange={setModalidade} ajuda="Ex.: US, CT" erro={tentou && !modOk ? 'Use 2 a 4 letras.' : null} />
      </div>
      <AreaDeTexto rotulo="Texto do modelo" value={texto} onChange={setTexto} linhas={8} ajuda="Use campos variáveis entre colchetes, como [medida]. Sem dado de paciente." erro={tentou && !textoOk ? 'Escreva o texto do modelo.' : null} />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="criar o modelo" />}
      <div className="row" style={{ gap: 8 }}>
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          {tipo === 'clinica' ? 'Enviar para aprovação' : 'Criar e publicar'}
        </Botao>
        <Botao onPress={aoFechar}>Voltar</Botao>
      </div>
    </form>
  );
}
