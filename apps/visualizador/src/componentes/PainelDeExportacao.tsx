// Painel à direita: imagens marcadas, tamanho da página e os botões de PDF e JPEG (VIS-07, VIS-08).
// "Um PDF, um paciente": com exames diferentes marcados o botão fica apagado com o motivo (R1);
// o Rust confere de novo (VIS-S1).
import { Botao, BotaoDeIcone, GrupoDeOpcoes } from '@vidacor/design-system';
import type { Estudo, Item } from '../ponte';
import { estudosNaSelecao, LIMITE_DE_ITENS, rotuloDaJanela } from '../estado';

export interface PropsDoPainel {
  estudos: Estudo[];
  selecao: Item[];
  porPagina: number;
  aoMudarPorPagina: (n: number) => void;
  aoTirar: (item: Item) => void;
  aoLimpar: () => void;
  aoGerarPdf: () => void;
  aoSalvarJpeg: () => void;
  ocupado: boolean;
}

function descricao(estudos: Estudo[], i: Item): string {
  const serie = estudos[i.estudo]?.series[i.serie];
  const img = serie?.imagens[i.imagem];
  const partes = [
    serie ? `Série ${String(serie.numero ?? i.serie + 1)}` : 'Série',
    `imagem ${String(img?.numero ?? i.imagem + 1)}`,
  ];
  if (img && img.quadros > 1) {
    partes.push(`quadro ${String(i.quadro + 1)}`);
  }
  return partes.join(', ');
}

export function PainelDeExportacao(p: PropsDoPainel) {
  const estudos = estudosNaSelecao(p.selecao);
  const misturado = estudos.length > 1;
  const vazio = p.selecao.length === 0;
  const paginas = Math.ceil(p.selecao.length / p.porPagina);
  return (
    <aside className="vz-painel" aria-label="Exportar">
      <h2 className="vz-titulo">Para exportar</h2>
      <p className="t13 mute" style={{ margin: 0 }}>
        {vazio
          ? 'Nenhuma imagem marcada. Marque com M, com o botão abaixo da imagem ou pelo menu de contexto.'
          : `${String(p.selecao.length)} de no máximo ${String(LIMITE_DE_ITENS)} imagens`}
      </p>
      <ol className="vz-marcadas">
        {p.selecao.map((i) => (
          <li key={`${String(i.estudo)}-${String(i.serie)}-${String(i.imagem)}-${String(i.quadro)}`}>
            <span className="vz-marcada-txt">
              <span>{descricao(p.estudos, i)}</span>
              <span className="t13 mute">{i.janela ? `Janela ${rotuloDaJanela(i.janela)}` : 'Cor original'}</span>
            </span>
            <BotaoDeIcone
              icone="fechar"
              estilo="fechar"
              rotulo={`Tirar ${descricao(p.estudos, i)} da exportação`}
              onPress={() => {
                p.aoTirar(i);
              }}
            />
          </li>
        ))}
      </ol>
      <GrupoDeOpcoes
        rotulo="Imagens por página do PDF"
        valor={String(p.porPagina)}
        aoMudar={(v) => {
          p.aoMudarPorPagina(Number(v));
        }}
        opcoes={[
          { valor: '1', rotulo: '1' },
          { valor: '4', rotulo: '4' },
          { valor: '6', rotulo: '6' },
          { valor: '12', rotulo: '12' },
        ]}
      />
      {!vazio && !misturado && (
        <p className="t13 mute" style={{ margin: 0 }}>
          O PDF terá {paginas} {paginas === 1 ? 'página' : 'páginas'}, com nome, nascimento, exame e data em todas.
        </p>
      )}
      {misturado && (
        <p className="vz-bloqueio" role="alert">
          As imagens marcadas são de exames diferentes. Um PDF leva um só exame de um só paciente: tire as imagens do
          outro exame ou gere um PDF de cada vez.
        </p>
      )}
      <div className="col" style={{ gap: 8 }}>
        <Botao variante="principal" larguraTotal isDisabled={vazio || misturado} ocupado={p.ocupado} onPress={p.aoGerarPdf}>
          Gerar PDF do paciente
        </Botao>
        <Botao larguraTotal isDisabled={vazio} ocupado={p.ocupado} onPress={p.aoSalvarJpeg}>
          Salvar JPEG
        </Botao>
        <Botao variante="discreto" larguraTotal isDisabled={vazio || p.ocupado} onPress={p.aoLimpar}>
          Desmarcar todas
        </Botao>
      </div>
    </aside>
  );
}
