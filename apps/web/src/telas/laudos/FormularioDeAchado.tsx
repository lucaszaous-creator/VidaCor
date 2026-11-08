import type {
  AchadoEstruturado,
  CategoriaBirads,
  ComposicaoTirads,
  EcogenicidadeTirads,
  FocoTirads,
  FormaBirads,
  FormaTirads,
  LadoBirads,
  MargemBirads,
  MargemTirads,
  NovoAchado,
  OrientacaoBirads,
  PadraoEcogenicoBirads,
  PosteriorBirads,
  SistemaClassificacao,
} from '@vidacor/api-client';
import { Aviso, Botao, CaixaDeMarcar, Campo, GrupoDeOpcoes, OpcoesMultiplas } from '@vidacor/design-system';
import { useState } from 'react';
import { ErroDoAto } from '../../componentes/ErroDoAto';
import { useDependencias } from '../../contexto';
import { useOperacao } from '../../operacao';
import {
  CATEGORIA_BIRADS,
  COMPOSICAO,
  ECOGENICIDADE,
  FOCOS,
  FORMA_BIRADS,
  FORMA_TIRADS,
  LADO,
  MARGEM_BIRADS,
  MARGEM_TIRADS,
  ORIENTACAO,
  PADRAO,
  POSTERIOR,
} from './opcoesDosAchados';

const SISTEMAS = [
  { valor: 'tirads', rotulo: 'TI-RADS (tireoide)' },
  { valor: 'birads', rotulo: 'BI-RADS (mama)' },
];

interface Tirads {
  composicao: ComposicaoTirads | null;
  ecogenicidade: EcogenicidadeTirads | null;
  forma: FormaTirads | null;
  margem: MargemTirads | null;
  focos: FocoTirads[];
}

interface Birads {
  lado: LadoBirads | null;
  categoria: CategoriaBirads | null;
  biopsia: boolean;
  comMassa: boolean;
  forma: FormaBirads | null;
  orientacao: OrientacaoBirads | null;
  margens: MargemBirads[];
  padrao: PadraoEcogenicoBirads | null;
  posterior: PosteriorBirads | null;
}

const TIRADS_VAZIO: Tirads = { composicao: null, ecogenicidade: null, forma: null, margem: null, focos: [] };
const BIRADS_VAZIO: Birads = {
  lado: null,
  categoria: null,
  biopsia: false,
  comMassa: true,
  forma: null,
  orientacao: null,
  margens: [],
  padrao: null,
  posterior: null,
};

/** "18" ou "8,5" em milímetros; `undefined` quando não é número (a ajuda de formato aparece). */
function lerMilimetros(texto: string): number | null | undefined {
  const t = texto.trim();
  if (t === '') return null;
  if (!/^\d{1,3}([.,]\d)?$/.test(t)) return undefined;
  return Number(t.replace(',', '.'));
}

/**
 * Monta o corpo ou diz o que falta marcar (completude do formulário, não regra clínica: coerência,
 * categoria e conduta são do servidor).
 */
function montar(sistema: SistemaClassificacao, local: string, mm: number | null, t: Tirads, b: Birads): NovoAchado | string {
  if (local.trim() === '') return 'Informe a localização do achado.';
  if (sistema === 'tirads') {
    const { composicao, ecogenicidade, forma, margem } = t;
    if (composicao === null || ecogenicidade === null || forma === null || margem === null) {
      return 'Marque composição, ecogenicidade, forma e margens.';
    }
    return { sistema, localizacao: local, maior_diametro_mm: mm, tirads: { composicao, ecogenicidade, forma, margem, focos: t.focos } };
  }
  const { lado, categoria, forma, orientacao, padrao, posterior } = b;
  if (lado === null || categoria === null) return 'Marque o lado e a categoria BI-RADS.';
  let massa = null;
  if (b.comMassa) {
    if (forma === null || orientacao === null || b.margens.length === 0 || padrao === null || posterior === null) {
      return 'Descreva a massa: forma, orientação, margens, padrão ecogênico e achados posteriores.';
    }
    massa = { forma, orientacao, margens: b.margens, padrao_ecogenico: padrao, posterior };
  }
  return {
    sistema,
    localizacao: local,
    maior_diametro_mm: b.comMassa ? mm : null,
    birads: { lado, categoria_atribuida: categoria, biopsia_maligna_confirmada: b.biopsia, massa },
  };
}

export interface PropsDoFormularioDeAchado {
  laudoId: string;
  aoRegistrar: (achado: AchadoEstruturado) => void | Promise<void>;
}

/**
 * Novo achado estruturado (Laudo-Estruturado.dc.html, "Novo nódulo"). A tela só coleta os
 * descritores: pontos, categoria e conduta vêm na resposta do servidor (G1). A chave de
 * idempotência nasce com o formulário e se repete em toda retentativa (G3).
 */
export function FormularioDeAchado({ laudoId, aoRegistrar }: PropsDoFormularioDeAchado) {
  const { api } = useDependencias();
  const [sistema, setSistema] = useState<SistemaClassificacao>('tirads');
  const [local, setLocal] = useState('');
  const [medida, setMedida] = useState('');
  const [t, setT] = useState<Tirads>(TIRADS_VAZIO);
  const [b, setB] = useState<Birads>(BIRADS_VAZIO);
  const [tentou, setTentou] = useState(false);
  const op = useOperacao((corpo: NovoAchado, chave: string) => api.laudos.criarAchado(laudoId, corpo, { idempotencyKey: chave }), {
    aoConcluir: async (a) => {
      setLocal('');
      setMedida('');
      setT(TIRADS_VAZIO);
      setB(BIRADS_VAZIO);
      setTentou(false);
      op.reiniciar();
      await aoRegistrar(a);
    },
  });

  const mm = lerMilimetros(medida);
  const erroDeFormato = mm === undefined ? 'Use números em milímetros, por exemplo 18 ou 8,5.' : null;
  const montado = montar(sistema, local, mm ?? null, t, b);
  const falta = typeof montado === 'string' ? montado : null;

  function registrar(): void {
    setTentou(true);
    if (typeof montado === 'string' || erroDeFormato !== null) return;
    op.enviar(montado);
  }

  const mudaT = <K extends keyof Tirads>(k: K) => (v: Tirads[K]) => {
    setT((a) => ({ ...a, [k]: v }));
    op.limparErro();
  };
  const mudaB = <K extends keyof Birads>(k: K) => (v: Birads[K]) => {
    setB((a) => ({ ...a, [k]: v }));
    op.limparErro();
  };

  return (
    <section className="col" style={{ gap: 8 }} aria-label="Novo achado estruturado">
      <h4 className="t14 b">Novo achado</h4>
      <GrupoDeOpcoes
        rotulo="Classificação"
        valor={sistema}
        opcoes={SISTEMAS}
        aoMudar={(v) => {
          setSistema(v === 'birads' ? 'birads' : 'tirads');
          op.limparErro();
        }}
      />
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 2fr) minmax(0, 1fr)', gap: 12 }}>
        <Campo
          rotulo="Localização"
          value={local}
          onChange={setLocal}
          maxLength={120}
          placeholder={sistema === 'tirads' ? 'Lobo direito, terço médio' : 'Quadrante superolateral, 2 horas'}
        />
        <Campo
          rotulo="Maior diâmetro (mm)"
          value={medida}
          onChange={setMedida}
          inputMode="decimal"
          erro={tentou ? erroDeFormato : null}
          isDisabled={sistema === 'birads' && !b.comMassa}
        />
      </div>
      {sistema === 'tirads' ? (
        <div>
          <GrupoDeOpcoes rotulo="Composição" valor={t.composicao} opcoes={COMPOSICAO} aoMudar={(v) => { mudaT('composicao')(v as ComposicaoTirads); }} />
          <GrupoDeOpcoes rotulo="Ecogenicidade" valor={t.ecogenicidade} opcoes={ECOGENICIDADE} aoMudar={(v) => { mudaT('ecogenicidade')(v as EcogenicidadeTirads); }} />
          <GrupoDeOpcoes rotulo="Forma" valor={t.forma} opcoes={FORMA_TIRADS} aoMudar={(v) => { mudaT('forma')(v as FormaTirads); }} />
          <GrupoDeOpcoes rotulo="Margens" valor={t.margem} opcoes={MARGEM_TIRADS} aoMudar={(v) => { mudaT('margem')(v as MargemTirads); }} />
          <OpcoesMultiplas rotulo="Focos ecogênicos" valores={t.focos} opcoes={FOCOS} aoMudar={(v) => { mudaT('focos')(v as FocoTirads[]); }} />
        </div>
      ) : (
        <div>
          <GrupoDeOpcoes rotulo="Lado" valor={b.lado} opcoes={LADO} aoMudar={(v) => { mudaB('lado')(v as LadoBirads); }} />
          <GrupoDeOpcoes rotulo="Categoria BI-RADS" valor={b.categoria} opcoes={CATEGORIA_BIRADS} aoMudar={(v) => { mudaB('categoria')(v as CategoriaBirads); }} />
          <CaixaDeMarcar rotulo="Massa descrita" detalhe="Desmarque em BI-RADS 1 (negativo) ou 0 sem massa." marcado={b.comMassa} aoMudar={mudaB('comMassa')} />
          {b.comMassa && (
            <>
              <GrupoDeOpcoes rotulo="Forma" valor={b.forma} opcoes={FORMA_BIRADS} aoMudar={(v) => { mudaB('forma')(v as FormaBirads); }} />
              <GrupoDeOpcoes rotulo="Orientação" valor={b.orientacao} opcoes={ORIENTACAO} aoMudar={(v) => { mudaB('orientacao')(v as OrientacaoBirads); }} />
              <OpcoesMultiplas rotulo="Margens" valores={b.margens} opcoes={MARGEM_BIRADS} aoMudar={(v) => { mudaB('margens')(v as MargemBirads[]); }} />
              <GrupoDeOpcoes rotulo="Padrão ecogênico" valor={b.padrao} opcoes={PADRAO} aoMudar={(v) => { mudaB('padrao')(v as PadraoEcogenicoBirads); }} />
              <GrupoDeOpcoes rotulo="Achados posteriores" valor={b.posterior} opcoes={POSTERIOR} aoMudar={(v) => { mudaB('posterior')(v as PosteriorBirads); }} />
            </>
          )}
          <CaixaDeMarcar rotulo="Malignidade confirmada por biópsia" marcado={b.biopsia} aoMudar={mudaB('biopsia')} />
        </div>
      )}
      {tentou && falta !== null && <Aviso nivel="atencao" titulo={falta} />}
      {op.erro !== null && <ErroDoAto erro={op.erro} ato="registrar o achado" aoTentarDeNovo={registrar} />}
      <div className="row">
        <Botao variante="azul" ocupado={op.enviando} onPress={registrar}>
          Registrar achado
        </Botao>
      </div>
    </section>
  );
}
