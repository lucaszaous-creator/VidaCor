import { useQuery } from '@tanstack/react-query';
import type { Laudo } from '@vidacor/api-client';
import { Aviso, Botao, Campo, Escolha } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

function Conteudo({ laudo, aoInserir }: { laudo: Laudo; aoInserir: (texto: string) => void }) {
  const { api } = useDependencias();
  const modelos = useQuery({ queryKey: ['modelos-de-laudo'], queryFn: () => api.modelosDeLaudo.listar() });
  const frases = useQuery({ queryKey: ['frases-de-laudo'], queryFn: () => api.frasesDeLaudo.listar() });
  const [modelo, setModelo] = useState('');
  const [atalho, setAtalho] = useState('');
  const disponiveis = (modelos.data ?? []).filter((m) => m.ativo && m.versao_publicada !== null && m.versao_publicada !== undefined);
  const op = useOperacao(
    (p: { modelo_id: string; numero: number }, chave: string) => api.laudos.usarModelo(laudo.id, p, { idempotencyKey: chave }),
    {
      aoConcluir: (u) => {
        aoInserir(u.texto);
      },
    },
  );
  const frase = (frases.data ?? []).find((f) => f.atalho === atalho.trim().toLowerCase());
  return (
    <div className="col" style={{ gap: 10 }}>
      <EstadoDaConsulta oQue="os modelos" carregando={modelos.isPending} erro={modelos.error} aoTentarDeNovo={() => void modelos.refetch()} linhas={1} />
      <div className="row wrap" style={{ gap: 8, alignItems: 'flex-end' }}>
        <div style={{ flex: '1 1 260px', minWidth: 0 }}>
          <Escolha
            rotulo="Modelo"
            valor={modelo}
            aoMudar={(v) => {
              setModelo(v);
              op.reiniciar();
            }}
            vazio={disponiveis.length === 0 ? 'Nenhum modelo publicado' : 'Escolha o modelo'}
            opcoes={disponiveis.map((m) => ({ valor: m.id, rotulo: `${m.nome} (${m.autor}, v${String(m.versao_publicada ?? '')})` }))}
          />
        </div>
        <Botao
          pequeno
          ocupado={op.enviando}
          isDisabled={modelo === ''}
          onPress={() => {
            const m = disponiveis.find((x) => x.id === modelo);
            if (m?.versao_publicada !== null && m?.versao_publicada !== undefined) op.enviar({ modelo_id: m.id, numero: m.versao_publicada });
          }}
        >
          Inserir modelo
        </Botao>
      </div>
      <div aria-live="polite">
        {op.resultado !== null && (
          <Aviso nivel="feito" titulo={`Modelo ${op.resultado.modelo_codigo} v${String(op.resultado.numero)} inserido no rascunho.`}>
            Preencha os campos entre colchetes. A versão do modelo ficou registrada neste laudo.
          </Aviso>
        )}
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="inserir o modelo" />}
      </div>
      <div className="row wrap" style={{ gap: 8, alignItems: 'flex-end' }}>
        <div style={{ width: 200 }}>
          <Campo rotulo="Atalho da frase" value={atalho} onChange={setAtalho} placeholder=".normal" />
        </div>
        <Botao
          pequeno
          isDisabled={frase === undefined}
          onPress={() => {
            if (frase !== undefined) {
              aoInserir(frase.texto);
              setAtalho('');
            }
          }}
        >
          Inserir frase
        </Botao>
      </div>
      <p className="t13" role="status" style={{ margin: 0 }}>
        {atalho.trim() === '' ? '' : frase === undefined ? 'Nenhuma frase com esse atalho. Atalhos começam com ponto.' : frase.texto}
      </p>
    </div>
  );
}

/**
 * Modelos e frases no rascunho do laudo (Modelos-Laudo.dc.html). Fica fechado e só consulta o
 * servidor quando o médico abre. O texto entra no rascunho pelo salvamento normal do laudo.
 */
export function PainelDeModelos({ laudo, aoInserir }: { laudo: Laudo; aoInserir: (texto: string) => void }) {
  const [aberto, setAberto] = useState(false);
  return (
    <details
      style={{ borderTop: '1px solid var(--line)', paddingTop: 8 }}
      onToggle={(e) => {
        setAberto(e.currentTarget.open);
      }}
    >
      <summary className="t15 b" style={{ cursor: 'pointer', minHeight: 44, display: 'flex', alignItems: 'center' }}>
        Modelos e frases
      </summary>
      {aberto && <Conteudo laudo={laudo} aoInserir={aoInserir} />}
    </details>
  );
}
