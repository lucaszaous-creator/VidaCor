import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import type { Caixa, FechamentoDeCaixa } from '@vidacor/api-client';
import {
  Aviso,
  Botao,
  Campo,
  Chip,
  DialogoDeFormulario,
  formatarDinheiro,
  formatarHora,
  ListaDeDados,
  lerReais,
} from '@vidacor/design-system';
import { useState } from 'react';
import { CabecalhoDaTela, EstadoDaConsulta } from '../../componentes/Bloco';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { chaves, consultaMeuCaixa } from '../../consultas';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import { FUSO_DA_CLINICA, hojeNaClinica } from '../agenda/grade';
import { FormularioDeRecebimento } from './FormularioDeRecebimento';

const rota = getRouteApi('/app/caixa');

function AbrirCaixa() {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const [fundo, setFundo] = useState('');
  const [tentou, setTentou] = useState(false);
  const centavos = lerReais(fundo);
  const op = useOperacao((c: number, chave: string) => api.caixas.abrir(c, { idempotencyKey: chave }), {
    aoConcluir: (c) => {
      queryClient.setQueryData(chaves.meuCaixa, c);
    },
  });
  return (
    <form
      className="col"
      style={{ gap: 12, maxWidth: 420 }}
      noValidate
      onSubmit={(ev) => {
        ev.preventDefault();
        setTentou(true);
        if (centavos !== null) op.enviar(centavos);
      }}
    >
      <Aviso nivel="atencao" titulo="Não há caixa aberto em seu nome.">
        Recebimentos só entram com o caixa aberto. Conte o troco da gaveta e abra o caixa.
      </Aviso>
      <Campo
        rotulo="Fundo de troco (R$)"
        value={fundo}
        onChange={setFundo}
        inputMode="decimal"
        ajuda="Ex.: 200,00"
        erro={tentou && centavos === null ? 'Digite o valor em reais, ex.: 200,00.' : null}
      />
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="abrir o caixa" aoTentarDeNovo={() => { if (centavos !== null) op.enviar(centavos); }} />}
      <div className="row">
        <Botao type="submit" variante="principal" ocupado={op.enviando}>
          Abrir caixa
        </Botao>
      </div>
    </form>
  );
}

function Fechamento({ f }: { f: FechamentoDeCaixa }) {
  const d = f.diferenca_centavos;
  const leitura = d === 0 ? 'Sem diferença' : d < 0 ? `Falta de ${formatarDinheiro(-d)}` : `Sobra de ${formatarDinheiro(d)}`;
  return (
    <div className="col" style={{ gap: 12, maxWidth: 520 }}>
      <Aviso nivel={d === 0 ? 'feito' : 'atencao'} titulo="Caixa fechado.">
        {leitura}. Os valores são os do servidor; a diferença fica registrada com o seu nome.
      </Aviso>
      <ListaDeDados
        larguraDoRotulo={160}
        itens={[
          { rotulo: 'Esperado', valor: <span className="num">{formatarDinheiro(f.esperado_centavos)}</span> },
          { rotulo: 'Contado', valor: <span className="num">{formatarDinheiro(f.contado_centavos)}</span> },
          { rotulo: 'Diferença', valor: <b className="num">{formatarDinheiro(d)}</b> },
        ]}
      />
    </div>
  );
}

function BotaoFecharCaixa({ caixa, aoFechar }: { caixa: Caixa; aoFechar: (f: FechamentoDeCaixa) => void }) {
  const { api } = useDependencias();
  const queryClient = useQueryClient();
  const [aberto, setAberto] = useState(false);
  const [contado, setContado] = useState('');
  const [tentou, setTentou] = useState(false);
  const centavos = lerReais(contado);
  const op = useOperacao((c: number, chave: string) => api.caixas.fechar(caixa.id, c, { idempotencyKey: chave }), {
    aoConcluir: (f) => {
      setAberto(false);
      aoFechar(f);
      queryClient.setQueryData(chaves.meuCaixa, null);
    },
  });
  return (
    <>
      <Botao
        pequeno
        onPress={() => {
          setAberto(true);
        }}
      >
        Fechar caixa
      </Botao>
      <DialogoDeFormulario
        aberto={aberto}
        irreversivel
        titulo={`Fechar o caixa ${caixa.codigo}?`}
        rotuloConfirmar="Fechar caixa"
        ocupado={op.enviando}
        aoFechar={() => {
          setAberto(false);
        }}
        aoConfirmar={() => {
          setTentou(true);
          if (centavos !== null) op.enviar(centavos);
        }}
      >
        <p className="t14" style={{ margin: 0 }}>
          Conte o dinheiro da gaveta e digite o total. O servidor compara com o esperado e mostra a diferença, sem esconder.
        </p>
        <Campo
          rotulo="Total contado na gaveta (R$)"
          value={contado}
          onChange={setContado}
          inputMode="decimal"
          erro={tentou && centavos === null ? 'Digite o valor em reais, ex.: 740,00.' : null}
        />
        {op.erro !== null && <ErroDoAto erro={op.erro} ato="fechar o caixa" aoTentarDeNovo={() => { if (centavos !== null) op.enviar(centavos); }} />}
      </DialogoDeFormulario>
    </>
  );
}

/** Caixa do particular (Caixa.dc.html, FC-CXA e FC-PIX). */
export function TelaCaixa() {
  const { api } = useDependencias();
  const busca = rota.useSearch();
  const data = busca.data ?? hojeNaClinica();
  const meu = useQuery(consultaMeuCaixa(api));
  const [fechamento, setFechamento] = useState<FechamentoDeCaixa | null>(null);
  const caixa = meu.data;

  return (
    <>
      <CabecalhoDaTela
        titulo={caixa === undefined || caixa === null ? 'Caixa' : `Caixa ${caixa.codigo}`}
        complemento={
          caixa === undefined || caixa === null ? undefined : (
            <>
              <Chip cor="verde" texto="Aberto" />
              <span className="t14 mute">
                desde {formatarHora(caixa.aberto_em, FUSO_DA_CLINICA)}, fundo de {formatarDinheiro(caixa.fundo_centavos)}
              </span>
            </>
          )
        }
      >
        {caixa !== undefined && caixa !== null && <BotaoFecharCaixa caixa={caixa} aoFechar={setFechamento} />}
      </CabecalhoDaTela>
      <section className="card pane fill" aria-label="Recebimento">
        <div className="scroll col" style={{ gap: 14 }}>
          <EstadoDaConsulta
            oQue="o caixa"
            carregando={meu.isPending}
            erro={meu.error}
            aoTentarDeNovo={() => void meu.refetch()}
            semPermissao={{ titulo: 'Você não opera caixa', detalhe: 'Peça à coordenação o perfil de caixa.' }}
          />
          {fechamento !== null && <Fechamento f={fechamento} />}
          {caixa === null && fechamento === null && <AbrirCaixa />}
          {caixa !== undefined && caixa !== null && (
            <FormularioDeRecebimento
              data={data}
              aoAbrirCaixa={() => {
                void meu.refetch();
              }}
            />
          )}
        </div>
      </section>
    </>
  );
}
