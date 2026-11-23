// Painel à direita (DEC-2026-11-23-16): o PDF do exame leva todas as imagens das séries marcadas
// (todas começam marcadas, como no PDF que a clínica entrega hoje); embaixo, as imagens avulsas
// marcadas para salvar em JPEG. O Rust confere tudo de novo (G1, VIS-S1).
import { Botao, BotaoDeIcone, CaixaDeMarcar } from '@vidacor/design-system';
import type { Estudo, Item } from '../ponte';
import { LIMITE_DE_ITENS, rotuloDaJanela } from '../estado';

export interface PropsDoPainel {
  estudo: Estudo;
  /** Séries do exame atual que vão para o PDF. */
  seriesNoPdf: number[];
  aoMarcarSerie: (serie: number, marcada: boolean) => void;
  aoGerarPdf: () => void;
  selecao: Item[];
  estudos: Estudo[];
  aoTirar: (item: Item) => void;
  aoLimpar: () => void;
  aoSalvarJpeg: () => void;
  ocupado: boolean;
}

function plural(n: number, um: string, varios: string): string {
  return `${String(n)} ${n === 1 ? um : varios}`;
}

function descricao(estudos: Estudo[], i: Item): string {
  const serie = estudos[i.estudo]?.series[i.serie];
  const img = serie?.imagens[i.imagem];
  const partes = [serie ? `Série ${String(serie.numero ?? i.serie + 1)}` : 'Série', `imagem ${String(img?.numero ?? i.imagem + 1)}`];
  if (img && img.quadros > 1) {
    partes.push(`quadro ${String(i.quadro + 1)}`);
  }
  return partes.join(', ');
}

export function PainelDeExportacao(p: PropsDoPainel) {
  const marcadas = p.estudo.series.map((s, i) => ({ s, i })).filter(({ i }) => p.seriesNoPdf.includes(i));
  const imagens = marcadas.reduce((t, { s }) => t + s.imagens.length, 0);
  const paginas = marcadas.reduce((t, { s }) => t + s.paginasNoPdf, 0);
  const excede = imagens > LIMITE_DE_ITENS;
  return (
    <aside className="vz-painel" aria-label="Exportar">
      <section className="vz-bloco vz-bloco-pdf" aria-label="PDF do exame">
        <h2 className="vz-titulo">PDF do exame</h2>
        <p className="t13 mute" style={{ margin: 0 }}>
          Todas as imagens das séries marcadas, uma série por página, com nome e nascimento em todas as páginas.
        </p>
        <ul className="vz-series-pdf">
          {p.estudo.series.map((s, i) => (
            <li key={`${String(s.numero)}-${String(i)}`}>
              <CaixaDeMarcar
                rotulo={`Série ${String(s.numero ?? i + 1)}${s.descricao ? `, ${s.descricao}` : ''}`}
                detalhe={`${plural(s.imagens.length, 'imagem', 'imagens')}, ${plural(s.paginasNoPdf, 'página', 'páginas')}`}
                marcado={p.seriesNoPdf.includes(i)}
                aoMudar={(m) => {
                  p.aoMarcarSerie(i, m);
                }}
              />
            </li>
          ))}
        </ul>
        {marcadas.length > 0 && !excede && (
          <p className="t13" style={{ margin: 0 }}>
            O PDF terá {plural(imagens, 'imagem', 'imagens')} em {plural(paginas, 'página', 'páginas')}.
          </p>
        )}
        {excede && (
          <p className="vz-bloqueio" role="alert">
            São {String(imagens)} imagens; o limite é {String(LIMITE_DE_ITENS)} por PDF. Desmarque as séries de corte fino ou
            gere um PDF por parte.
          </p>
        )}
        <Botao variante="principal" larguraTotal isDisabled={marcadas.length === 0 || excede} ocupado={p.ocupado} onPress={p.aoGerarPdf}>
          Gerar PDF do exame
        </Botao>
      </section>

      <section className="vz-bloco vz-bloco-jpeg" aria-label="Imagens avulsas em JPEG">
        <h2 className="vz-titulo">Imagens avulsas em JPEG</h2>
        {p.selecao.length === 0 ? (
          <p className="t13 mute" style={{ margin: 0 }}>
            Nenhuma marcada. Marque com M, com o botão abaixo da imagem ou pelo menu de contexto.
          </p>
        ) : (
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
                  rotulo={`Tirar ${descricao(p.estudos, i)} dos JPEG`}
                  onPress={() => {
                    p.aoTirar(i);
                  }}
                />
              </li>
            ))}
          </ol>
        )}
        <div className="col" style={{ gap: 6 }}>
          <Botao larguraTotal isDisabled={p.selecao.length === 0} ocupado={p.ocupado} onPress={p.aoSalvarJpeg}>
            Salvar JPEG
          </Botao>
          <Botao variante="discreto" larguraTotal isDisabled={p.selecao.length === 0 || p.ocupado} onPress={p.aoLimpar}>
            Desmarcar todas
          </Botao>
        </div>
      </section>
    </aside>
  );
}
