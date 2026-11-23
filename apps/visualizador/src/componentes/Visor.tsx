// Visor da imagem (VIS-04, VIS-05, VIS-14): roda do mouse troca o corte, arrastar ajusta a janela
// ou move a imagem, Ctrl + roda amplia. Os pixels vêm do Rust já com a janela aplicada, sem perda:
// o que aparece é o que o JPEG e o PDF terão (FC-VIS-05).
import { BotaoDeIcone, Botao, GrupoDeOpcoes, MenuDeContexto, useMenuDeContexto, type AcaoDoMenu } from '@vidacor/design-system';
import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type WheelEvent } from 'react';
import { mensagemDeErro, type InfoQuadro, type Janela, type Pixels, type Ponte, type Posicao } from '../ponte';
import { rotuloDaJanela } from '../estado';

export interface PropsDoVisor {
  ponte: Ponte;
  abertura: number;
  posicao: Posicao;
  /** Rótulos da posição atual: "Imagem 3 de 35", "Quadro 2 de 4". */
  rotulos: string[];
  descricaoDaSerie: string;
  total: number;
  indice: number;
  aoIrPara: (indice: number) => void;
  /** Janela escolhida; nula = a do aparelho. */
  janela: Janela | null;
  aoMudarJanela: (j: Janela | null) => void;
  marcada: boolean;
  totalMarcadas: number;
  aoAlternarMarca: (janelaEfetiva: Janela | null) => void;
  aoSalvarJpeg: (janelaEfetiva: Janela | null) => void;
  paciente: { nome: string; nascimento: string };
}

type Ferramenta = 'contraste' | 'mover';

function iguais(a: Janela | null, b: Janela | null): boolean {
  return a !== null && b !== null && Math.abs(a.centro - b.centro) < 0.01 && Math.abs(a.largura - b.largura) < 0.01;
}

export function Visor(p: PropsDoVisor) {
  const { ponte, abertura, posicao, janela } = p;
  const [info, setInfo] = useState<InfoQuadro | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [ferramenta, setFerramenta] = useState<Ferramenta>('contraste');
  const [zoom, setZoom] = useState<number | null>(null);
  const [desloc, setDesloc] = useState({ x: 0, y: 0 });
  const [area, setArea] = useState({ largura: 1, altura: 1 });
  const [acoes, setAcoes] = useState<AcaoDoMenu[]>([]);
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const caixa = useRef<HTMLDivElement | null>(null);
  const arraste = useRef<{ x: number; y: number; janela: Janela; desloc: { x: number; y: number } } | null>(null);
  const menu = useMenuDeContexto();

  const efetiva = janela ?? info?.janelaPadrao ?? null;

  // Informação do quadro (tamanho e janelas) a cada troca de posição.
  useEffect(() => {
    let vivo = true;
    ponte
      .quadro(abertura, posicao)
      .then((i) => {
        if (vivo) {
          setInfo(i);
          setErro(null);
        }
      })
      .catch((e: unknown) => {
        if (vivo) {
          setInfo(null);
          setErro(mensagemDeErro(e));
        }
      });
    return () => {
      vivo = false;
    };
  }, [ponte, abertura, posicao]);

  // Pixels: só o pedido mais recente é desenhado; durante o arraste, um pedido por vez.
  const emCurso = useRef(false);
  const pendente = useRef<{ posicao: Posicao; janela: Janela | null } | null>(null);
  const desenhar = useCallback(
    (primeiro: { posicao: Posicao; janela: Janela | null }) => {
      const executar = (pedido: { posicao: Posicao; janela: Janela | null }) => {
        if (emCurso.current) {
          pendente.current = pedido;
          return;
        }
        emCurso.current = true;
        void ponte
          .pixels(abertura, pedido.posicao, pedido.janela)
          .then((px: Pixels) => {
            const c = canvas.current;
            const ctx = c?.getContext('2d');
            if (c && ctx) {
              c.width = px.largura;
              c.height = px.altura;
              ctx.putImageData(new ImageData(px.rgba, px.largura, px.altura), 0, 0);
            }
          })
          .catch((e: unknown) => {
            setErro(mensagemDeErro(e));
          })
          .finally(() => {
            emCurso.current = false;
            const prox = pendente.current;
            pendente.current = null;
            if (prox) {
              executar(prox);
            }
          });
      };
      executar(primeiro);
    },
    [ponte, abertura],
  );

  useEffect(() => {
    if (info) {
      desenhar({ posicao, janela: info.cor ? null : efetiva });
    }
  }, [info, posicao, efetiva, desenhar]);

  // Tamanho da área para o "ajustar à tela".
  useEffect(() => {
    const el = caixa.current;
    if (!el) {
      return undefined;
    }
    const obs = new ResizeObserver(([e]) => {
      if (e) {
        setArea({ largura: e.contentRect.width, altura: e.contentRect.height });
      }
    });
    obs.observe(el);
    return () => {
      obs.disconnect();
    };
  }, []);

  // Ações do menu de contexto vêm do Rust (docs/19).
  useEffect(() => {
    let vivo = true;
    ponte
      .acoesDaImagem(p.marcada, p.totalMarcadas)
      .then((a) => {
        if (vivo) {
          setAcoes(
            a.map((x) => ({
              id: x.id,
              rotulo: x.rotulo,
              estado: x.estado,
              ...(x.atalho ? { atalho: x.atalho } : {}),
              ...(x.motivo ? { motivo: x.motivo } : {}),
            })),
          );
        }
      })
      .catch(() => {
        if (vivo) {
          setAcoes([]);
        }
      });
    return () => {
      vivo = false;
    };
  }, [ponte, p.marcada, p.totalMarcadas]);

  const ajuste = info ? Math.min(area.largura / info.largura, area.altura / info.altura) : 1;
  const escala = zoom ?? ajuste;

  const ir = (delta: number) => {
    p.aoIrPara(Math.min(p.total - 1, Math.max(0, p.indice + delta)));
  };
  const ampliar = (fator: number) => {
    setZoom(Math.min(16, Math.max(0.1, escala * fator)));
  };
  const ajustarATela = () => {
    setZoom(null);
    setDesloc({ x: 0, y: 0 });
  };

  const aoRodar = (ev: WheelEvent<HTMLDivElement>) => {
    if (ev.ctrlKey) {
      ampliar(ev.deltaY < 0 ? 1.15 : 1 / 1.15);
    } else {
      ir(ev.deltaY > 0 ? 1 : -1);
    }
  };

  const aoApertar = (ev: PointerEvent<HTMLDivElement>) => {
    if (ev.button !== 0) {
      return;
    }
    ev.currentTarget.setPointerCapture(ev.pointerId);
    arraste.current = { x: ev.clientX, y: ev.clientY, janela: efetiva ?? { centro: 0, largura: 1 }, desloc };
  };
  const aoMover = (ev: PointerEvent<HTMLDivElement>) => {
    const a = arraste.current;
    if (!a) {
      return;
    }
    const dx = ev.clientX - a.x;
    const dy = ev.clientY - a.y;
    if (ferramenta === 'mover' || info?.cor === true) {
      setDesloc({ x: a.desloc.x + dx, y: a.desloc.y + dy });
      return;
    }
    // Horizontal muda a largura, vertical o centro; passo proporcional à janela atual.
    const passo = Math.max(a.janela.largura / 300, 0.5);
    p.aoMudarJanela({ centro: a.janela.centro + dy * passo, largura: Math.max(1, a.janela.largura + dx * passo) });
  };
  const aoSoltar = () => {
    arraste.current = null;
  };

  const escolherAcao = (id: string) => {
    if (id === 'marcar' || id === 'desmarcar') {
      p.aoAlternarMarca(info?.cor === true ? null : efetiva);
    } else if (id === 'salvar-jpeg') {
      p.aoSalvarJpeg(info?.cor === true ? null : efetiva);
    } else if (id === 'janela-padrao') {
      p.aoMudarJanela(null);
    }
  };

  const aoTeclar = (ev: KeyboardEvent<HTMLDivElement>) => {
    menu.propsDoAlvo.onKeyDown(ev);
    if (ev.defaultPrevented) {
      return;
    }
    const teclas: Record<string, () => void> = {
      ArrowUp: () => {
        ir(-1);
      },
      ArrowLeft: () => {
        ir(-1);
      },
      ArrowDown: () => {
        ir(1);
      },
      ArrowRight: () => {
        ir(1);
      },
      PageUp: () => {
        ir(-10);
      },
      PageDown: () => {
        ir(10);
      },
      Home: () => {
        p.aoIrPara(0);
      },
      End: () => {
        p.aoIrPara(p.total - 1);
      },
      m: () => {
        escolherAcao(p.marcada ? 'desmarcar' : 'marcar');
      },
      r: () => {
        p.aoMudarJanela(null);
      },
      '+': () => {
        ampliar(1.25);
      },
      '-': () => {
        ampliar(0.8);
      },
      '0': ajustarATela,
    };
    const prontas = info?.prontas ?? [];
    const n = Number(ev.key);
    const acao = teclas[ev.key.length === 1 ? ev.key.toLowerCase() : ev.key];
    if (acao) {
      ev.preventDefault();
      acao();
    } else if (n >= 1 && n <= prontas.length) {
      const pr = prontas[n - 1];
      if (pr) {
        ev.preventDefault();
        p.aoMudarJanela({ centro: pr.centro, largura: pr.largura });
      }
    }
  };

  const opcoesDeJanela: { rotulo: string; janela: Janela | null; dica?: string }[] = info?.cor
    ? []
    : [
        { rotulo: 'Do aparelho', janela: null, dica: info?.janelaPadrao ? rotuloDaJanela(info.janelaPadrao) : undefined },
        ...(info?.janelasDoArquivo.slice(1).map((j, i) => ({ rotulo: `Aparelho ${String(i + 2)}`, janela: j, dica: rotuloDaJanela(j) })) ?? []),
        ...(info?.prontas.map((pr, i) => ({
          rotulo: pr.nome,
          janela: { centro: pr.centro, largura: pr.largura },
          dica: `${rotuloDaJanela(pr)}, tecla ${String(i + 1)}`,
        })) ?? []),
      ];

  return (
    <section className="vz-visor" aria-label="Imagem do exame">
      <div className="vz-ferramentas">
        <div className="vz-janelas" role="group" aria-label="Janela">
          {opcoesDeJanela.map((o) => {
            const ativa = o.janela === null ? janela === null : iguais(janela, o.janela);
            return (
              <button
                key={o.rotulo}
                type="button"
                className={ativa ? 'vz-jan ativa' : 'vz-jan'}
                aria-pressed={ativa}
                title={o.dica}
                onClick={() => {
                  p.aoMudarJanela(o.janela);
                }}
              >
                {o.rotulo}
              </button>
            );
          })}
          {info?.cor === true && <span className="t13 mute">Imagem colorida: sem ajuste de janela</span>}
        </div>
        <GrupoDeOpcoes
          rotulo="Arrastar o mouse"
          valor={ferramenta}
          aoMudar={(v) => {
            setFerramenta(v === 'mover' ? 'mover' : 'contraste');
          }}
          opcoes={[
            { valor: 'contraste', rotulo: 'Contraste' },
            { valor: 'mover', rotulo: 'Mover' },
          ]}
        />
        <div className="row vz-zoom">
          <Botao pequeno onPress={() => { ampliar(0.8); }}>Reduzir</Botao>
          <Botao pequeno onPress={() => { ampliar(1.25); }}>Ampliar</Botao>
          <Botao pequeno onPress={ajustarATela}>Ajustar à tela</Botao>
        </div>
      </div>

      <div
        ref={caixa}
        className={ferramenta === 'mover' ? 'vz-area mover' : 'vz-area'}
        tabIndex={0}
        aria-label={`${p.descricaoDaSerie}, ${p.rotulos.join(', ')}. Setas trocam a imagem; M marca para exportar; menu de contexto com Shift+F10.`}
        onWheel={aoRodar}
        onPointerDown={aoApertar}
        onPointerMove={aoMover}
        onPointerUp={aoSoltar}
        onPointerCancel={aoSoltar}
        onContextMenu={menu.propsDoAlvo.onContextMenu}
        onKeyDown={aoTeclar}
        aria-haspopup="menu"
      >
        <canvas
          ref={canvas}
          className="vz-canvas"
          style={{ transform: `translate(-50%, -50%) translate(${String(desloc.x)}px, ${String(desloc.y)}px) scale(${String(escala)})` }}
        />
        <div className="vz-sobre vz-sobre-esq">
          <div>{p.descricaoDaSerie}</div>
          {p.rotulos.map((r) => (
            <div key={r}>{r}</div>
          ))}
        </div>
        <div className="vz-sobre vz-sobre-dir">
          {efetiva && info?.cor !== true && <div>Janela {rotuloDaJanela(efetiva)}</div>}
          <div>Zoom {Math.round(escala * 100)}%</div>
          {p.marcada && <div className="vz-marcada">Marcada para exportar</div>}
        </div>
        {erro !== null && (
          <div className="vz-erro" role="alert">
            {erro}
          </div>
        )}
      </div>

      <div className="vz-rodape">
        <input
          type="range"
          className="vz-barra"
          min={0}
          max={Math.max(0, p.total - 1)}
          value={p.indice}
          aria-label="Imagem da série"
          aria-valuetext={p.rotulos.join(', ')}
          disabled={p.total <= 1}
          onChange={(e) => {
            p.aoIrPara(Number(e.target.value));
          }}
        />
        <Botao
          variante={p.marcada ? 'padrao' : 'azul'}
          onPress={() => {
            escolherAcao(p.marcada ? 'desmarcar' : 'marcar');
          }}
        >
          {p.marcada ? 'Tirar da exportação (M)' : 'Marcar para exportar (M)'}
        </Botao>
        <BotaoDeIcone
          icone="maisOpcoes"
          estilo="mais"
          rotulo="Mais opções da imagem"
          onPress={() => {
            if (caixa.current) {
              menu.abrirEm(caixa.current);
            }
          }}
        />
      </div>
      <MenuDeContexto
        estado={menu}
        cabecalho={{ titulo: p.paciente.nome, subtitulo: `Nascimento ${p.paciente.nascimento}` }}
        acoes={acoes}
        motivoSemAcoes="As ações desta imagem não carregaram; feche o menu e tente de novo."
        aoEscolher={escolherAcao}
      />
    </section>
  );
}
