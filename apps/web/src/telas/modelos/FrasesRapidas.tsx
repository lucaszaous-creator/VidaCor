import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { FraseDeLaudo, NovaFraseDeLaudo } from '@vidacor/api-client';
import { Aviso, Botao, Busca, CaixaDeMarcar, Campo, Chip, Tabela } from '@vidacor/design-system';
import { useState } from 'react';
import { EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';

export const CHAVE_FRASES = ['frases-de-laudo'] as const;

function NovaFrase({ coordenacao }: { coordenacao: boolean }) {
  const { api } = useDependencias();
  const cliente = useQueryClient();
  const [atalho, setAtalho] = useState('.');
  const [texto, setTexto] = useState('');
  const [daClinica, setDaClinica] = useState(false);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((n: NovaFraseDeLaudo, chave: string) => api.frasesDeLaudo.criar(n, { idempotencyKey: chave }), {
    aoConcluir: () => cliente.invalidateQueries({ queryKey: CHAVE_FRASES }),
  });
  const atalhoOk = /^\.[a-z0-9_-]{2,20}$/.test(atalho.trim().toLowerCase());
  const textoOk = texto.trim().length >= 3;
  return (
    <form
      className="col"
      style={{ gap: 8 }}
      aria-label="Nova frase"
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (atalhoOk && textoOk) op.enviar({ tipo: daClinica ? 'clinica' : 'pessoal', atalho: atalho.trim().toLowerCase(), texto: texto.trim() });
      }}
    >
      <h3 className="t15 b" style={{ margin: 0 }}>
        Nova frase
      </h3>
      <div style={{ display: 'grid', gridTemplateColumns: '160px minmax(0, 1fr)', gap: 10, alignItems: 'start' }}>
        <Campo rotulo="Atalho" value={atalho} onChange={setAtalho} ajuda="Começa com ponto" erro={tentou && !atalhoOk ? 'Use ponto e 2 a 20 letras sem acento.' : null} />
        <Campo rotulo="Vira o texto" value={texto} onChange={setTexto} erro={tentou && !textoOk ? 'Escreva a frase.' : null} />
      </div>
      {coordenacao && <CaixaDeMarcar rotulo="Frase da clínica" detalhe="Vale para todos os médicos da unidade." marcado={daClinica} aoMudar={setDaClinica} />}
      <div aria-live="polite">
        {op.resultado !== null && <Aviso nivel="feito" titulo={`Frase ${op.resultado.atalho} criada.`}>No laudo, use o atalho ou insira pela lista.</Aviso>}
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="criar a frase" />}
      </div>
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Criar frase
        </Botao>
      </div>
    </form>
  );
}

/** Frases rápidas: atalho com ponto; as da clínica e as do próprio médico. */
export function FrasesRapidas({ coordenacao }: { coordenacao: boolean }) {
  const { api } = useDependencias();
  const frases = useQuery({ queryKey: CHAVE_FRASES, queryFn: () => api.frasesDeLaudo.listar() });
  const [teste, setTeste] = useState('');
  const achada: FraseDeLaudo | undefined = frases.data?.find((f) => f.atalho === teste.trim().toLowerCase());
  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 3fr) minmax(0, 2fr)', gap: 16, minHeight: 0, height: '100%' }}>
      <section className="col" aria-label="Frases rápidas" style={{ gap: 8, minHeight: 0, overflow: 'auto' }}>
        <EstadoDaConsulta oQue="as frases" carregando={frases.isPending} erro={frases.error} aoTentarDeNovo={() => void frases.refetch()} />
        {frases.data !== undefined && (
          <Tabela
            rotulo="Frases rápidas"
            vazio="Nenhuma frase ainda."
            colunas={[
              { id: 'atalho', rotulo: 'Atalho', semQuebra: true, cabecalhoDaLinha: true, celula: (f) => <code className="num">{f.atalho}</code> },
              { id: 'texto', rotulo: 'Vira', celula: (f) => f.texto },
              { id: 'dono', rotulo: 'De quem', semQuebra: true, celula: (f) => <Chip cor={f.tipo === 'clinica' ? 'marinho' : 'violeta'} texto={f.dono} /> },
            ]}
            linhas={frases.data}
          />
        )}
        <p className="t13 mute" style={{ margin: 0 }}>
          No laudo, abra Modelos e frases e insira a frase pelo atalho. Editar uma frase cria outra versão; a anterior fica guardada.
        </p>
      </section>
      <section className="col" aria-label="Criar e testar frases" style={{ gap: 14, minHeight: 0, overflow: 'auto' }}>
        <NovaFrase coordenacao={coordenacao} />
        <div className="col" style={{ gap: 6 }}>
          <h3 className="t15 b" style={{ margin: 0 }}>
            Experimente um atalho
          </h3>
          <Busca rotulo="Experimente um atalho" valor={teste} aoMudar={setTeste} placeholder=".normal" />
          <p className="t14" role="status" style={{ margin: 0 }}>
            {teste.trim() === '' ? '' : achada === undefined ? 'Nenhuma frase com esse atalho. Atalhos começam com ponto.' : achada.texto}
          </p>
        </div>
      </section>
    </div>
  );
}
