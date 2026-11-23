// Tela do Visualizador (docs/31 E2): topo com a barra do paciente, séries à esquerda, imagem no meio e
// exportação à direita. Cabe em 1440×900 e 1280×800 sem rolar a página; listas rolam dentro do painel.
import { Aviso, BarraDoPaciente, Botao, Dialogo, type NivelDoAviso } from '@vidacor/design-system';
import { useCallback, useMemo, useState } from 'react';
import logoCompacto from '../../../design/assets/marca/logo-compacto.png';
import { ListaDeSeries } from './componentes/ListaDeSeries';
import { PainelDeExportacao } from './componentes/PainelDeExportacao';
import { TelaInicial } from './componentes/TelaInicial';
import { Visor } from './componentes/Visor';
import { alternar, chave, estaMarcada, posicoesDaSerie } from './estado';
import { mensagemDeErro, type Item, type Janela, type Ponte, type Resumo } from './ponte';

interface AvisoNaTela {
  nivel: NivelDoAviso;
  titulo: string;
  texto: string;
}

function quantas(n: number): string {
  return n === 1 ? '1 imagem' : `${String(n)} imagens`;
}

export function Aplicacao({ ponte }: { ponte: Ponte }) {
  const [resumo, setResumo] = useState<Resumo | null>(null);
  const [abrindo, setAbrindo] = useState(false);
  const [aviso, setAviso] = useState<AvisoNaTela | null>(null);
  const [estudo, setEstudo] = useState(0);
  const [serie, setSerie] = useState(0);
  const [indice, setIndice] = useState(0);
  const [janela, setJanela] = useState<Janela | null>(null);
  const [selecao, setSelecao] = useState<Item[]>([]);
  const [porPagina, setPorPagina] = useState(6);
  const [exportando, setExportando] = useState(false);
  const [confirmarPdf, setConfirmarPdf] = useState(false);

  const abrir = useCallback(
    async (pedir: () => Promise<Resumo | null>) => {
      setAbrindo(true);
      try {
        const r = await pedir();
        if (r === null) {
          return;
        }
        const tinhaSelecao = selecao.length > 0;
        if (r.estudos.length === 0) {
          setAviso({
            nivel: 'atencao',
            titulo: 'Nenhuma imagem de exame foi encontrada.',
            texto: `${String(r.ignorados.length)} arquivo(s) ficaram de fora (não são DICOM, estão danificados ou não têm imagem). Confira se escolheu a pasta ou o arquivo certo.`,
          });
          return;
        }
        setResumo(r);
        setEstudo(0);
        setSerie(0);
        setIndice(0);
        setJanela(null);
        setSelecao([]);
        setAviso(
          tinhaSelecao
            ? { nivel: 'informativo', titulo: 'Outro exame aberto.', texto: 'As imagens marcadas do exame anterior foram desmarcadas.' }
            : null,
        );
      } catch (e) {
        setAviso({ nivel: 'bloqueante', titulo: 'O exame não abriu.', texto: mensagemDeErro(e) });
      } finally {
        setAbrindo(false);
      }
    },
    [selecao.length],
  );

  const atual = resumo?.estudos[estudo];
  const serieAtual = atual?.series[serie];
  const posicoes = useMemo(() => (serieAtual ? posicoesDaSerie(serieAtual) : []), [serieAtual]);
  const pos = posicoes[Math.min(indice, Math.max(0, posicoes.length - 1))];
  const posicao = useMemo(() => (pos ? { estudo, serie, imagem: pos.imagem, quadro: pos.quadro } : null), [estudo, serie, pos]);

  const exportarJpeg = async (itens: Item[]) => {
    if (!resumo) {
      return;
    }
    setExportando(true);
    try {
      const r = await ponte.exportarJpeg(resumo.abertura, itens);
      if (r) {
        setAviso({ nivel: 'feito', titulo: `${quantas(r.quantidade)} salva(s) em JPEG.`, texto: `Pasta: ${r.destino}. A exportação ficou registrada.` });
      }
    } catch (e) {
      setAviso({ nivel: 'bloqueante', titulo: 'Os JPEG não foram salvos.', texto: mensagemDeErro(e) });
    } finally {
      setExportando(false);
    }
  };

  const gerarPdf = async () => {
    if (!resumo) {
      return;
    }
    setExportando(true);
    try {
      const r = await ponte.gerarPdf(resumo.abertura, selecao, porPagina);
      setConfirmarPdf(false);
      if (r) {
        setAviso({ nivel: 'feito', titulo: `PDF com ${quantas(r.quantidade)} salvo.`, texto: `Arquivo: ${r.destino}. A exportação ficou registrada.` });
      }
    } catch (e) {
      setConfirmarPdf(false);
      setAviso({ nivel: 'bloqueante', titulo: 'O PDF não foi gerado.', texto: mensagemDeErro(e) });
    } finally {
      setExportando(false);
    }
  };

  if (!resumo || !atual || !serieAtual || !posicao) {
    return (
      <div className="vz vz-vazio">
        {aviso && (
          <div className="vz-aviso-topo">
            <Aviso nivel={aviso.nivel} titulo={aviso.titulo}>
              {aviso.texto}
            </Aviso>
          </div>
        )}
        <TelaInicial
          ocupado={abrindo}
          aoAbrirPasta={() => void abrir(ponte.abrirPasta)}
          aoAbrirArquivo={() => void abrir(ponte.abrirArquivo)}
        />
      </div>
    );
  }

  const nascimento = atual.paciente.nascimento ?? 'não informado';
  const imagemAtual = serieAtual.imagens[posicao.imagem];
  const rotulos = [
    `Imagem ${String(posicao.imagem + 1)} de ${String(serieAtual.imagens.length)}${imagemAtual?.numero != null ? ` (nº ${String(imagemAtual.numero)})` : ''}`,
    ...(imagemAtual && imagemAtual.quadros > 1 ? [`Quadro ${String(posicao.quadro + 1)} de ${String(imagemAtual.quadros)}`] : []),
  ];
  const pacienteDoPdf = resumo.estudos[selecao[0]?.estudo ?? estudo] ?? atual;

  return (
    <div className="vz">
      <header className="vz-topo">
        <img src={logoCompacto} alt="VidaCor Imagem" height={34} />
        <span className="vz-nome">Visualizador de exames</span>
        <div className="row" style={{ gap: 8, marginLeft: 'auto' }}>
          <Botao pequeno ocupado={abrindo} onPress={() => void abrir(ponte.abrirPasta)}>
            Abrir pasta
          </Botao>
          <Botao pequeno ocupado={abrindo} onPress={() => void abrir(ponte.abrirArquivo)}>
            Abrir arquivo
          </Botao>
        </div>
      </header>
      <div className="vz-pac">
        <BarraDoPaciente
          nome={atual.paciente.nome || 'Paciente sem nome no exame'}
          nascimento={nascimento}
          resumo={[atual.descricao || 'Exame sem descrição', atual.data, atual.atendimento && `Atendimento ${atual.atendimento}`, atual.solicitante && `Solicitante ${atual.solicitante}`]
            .filter(Boolean)
            .join(', ')}
        />
      </div>
      {aviso && (
        <div className="vz-aviso">
          <Aviso
            nivel={aviso.nivel}
            titulo={aviso.titulo}
            acoes={
              <Botao
                pequeno
                variante="discreto"
                onPress={() => {
                  setAviso(null);
                }}
              >
                Fechar aviso
              </Botao>
            }
          >
            {aviso.texto}
          </Aviso>
        </div>
      )}
      <div className="vz-corpo">
        <ListaDeSeries
          estudos={resumo.estudos}
          ignorados={resumo.ignorados}
          estudoAtual={estudo}
          serieAtual={serie}
          aoEscolher={(e, s) => {
            setEstudo(e);
            setSerie(s);
            setIndice(0);
            setJanela(null);
          }}
        />
        <Visor
          key={`${String(resumo.abertura)}-${String(estudo)}-${String(serie)}`}
          ponte={ponte}
          abertura={resumo.abertura}
          posicao={posicao}
          rotulos={rotulos}
          descricaoDaSerie={`${serieAtual.modalidade} ${serieAtual.descricao}`.trim()}
          total={posicoes.length}
          indice={Math.min(indice, posicoes.length - 1)}
          aoIrPara={setIndice}
          janela={janela}
          aoMudarJanela={setJanela}
          marcada={estaMarcada(selecao, posicao)}
          totalMarcadas={selecao.length}
          aoAlternarMarca={(j) => {
            setSelecao((s) => alternar(s, { ...posicao, janela: j }));
          }}
          aoSalvarJpeg={(j) => void exportarJpeg([{ ...posicao, janela: j }])}
          paciente={{ nome: atual.paciente.nome, nascimento }}
        />
        <PainelDeExportacao
          estudos={resumo.estudos}
          selecao={selecao}
          porPagina={porPagina}
          aoMudarPorPagina={setPorPagina}
          aoTirar={(i) => {
            setSelecao((s) => s.filter((x) => chave(x) !== chave(i)));
          }}
          aoLimpar={() => {
            setSelecao([]);
          }}
          aoGerarPdf={() => {
            setConfirmarPdf(true);
          }}
          aoSalvarJpeg={() => void exportarJpeg(selecao)}
          ocupado={exportando}
        />
      </div>
      <Dialogo
        aberto={confirmarPdf}
        titulo={`Gerar o PDF com ${quantas(selecao.length)} para este paciente`}
        paciente={{ nome: pacienteDoPdf.paciente.nome, nascimento: pacienteDoPdf.paciente.nascimento ?? 'não informado' }}
        rotuloConfirmar="Gerar PDF"
        ocupado={exportando}
        aoConfirmar={() => void gerarPdf()}
        aoFechar={() => {
          setConfirmarPdf(false);
        }}
      >
        <p style={{ margin: 0 }}>
          {pacienteDoPdf.descricao || 'Exame sem descrição'}
          {pacienteDoPdf.data !== null && ` de ${pacienteDoPdf.data}`}. O PDF sai com nome, nascimento, exame e data em todas
          as páginas; depois você escolhe onde salvar.
        </p>
      </Dialogo>
    </div>
  );
}
