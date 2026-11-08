# 10 · Assets e licenças

## Mapa anatômico realista (`design/assets/anatomia/`)

- **Origem:** malhas 3D do **BodyParts3D** (versão 3.0, convertida para STL no
  espelho [Kevin-Mattheus-Moerman/BodyParts3D](https://github.com/Kevin-Mattheus-Moerman/BodyParts3D)):
  pele, ossos (costelas, vértebras, esterno, clavículas, quadril, fêmur,
  mandíbula), cartilagens costais, coração, lobos pulmonares, traqueia, esôfago,
  aorta, carótidas, fígado, rins, baço, estômago, cólon, bexiga e encéfalo.
- **Licença:** *"BodyParts3D, © The Database Center for Life Science, licensed
  under CC Attribution-Share Alike 2.1 Japan"*. Os renders derivados seguem a
  mesma licença (CC BY-SA 2.1 JP) e o **crédito precisa aparecer** onde forem
  usados. Os mockups trazem a linha de crédito no visualizador.
- Citação: Mitsuhashi N. et al. *BodyParts3D: 3D structure database for
  anatomical concepts.* Nucleic Acids Res. 2009. doi:10.1093/nar/gkn613.
- **Como foi gerado:** [`tools/render-anatomia/`](../tools/render-anatomia/).
  O `anat.html` (three.js 0.169 + STLLoader) monta a cena: pele translúcida,
  materiais por órgão, vistas `full`, `busto` e `abdome`, e `focus=cardio|abdome`.
  O `shot4.cjs` renderiza no Chromium headless (Playwright) e devolve as
  coordenadas de cada órgão na tela, que viram os hotspots em %. Para rodar:
  `npm i three@0.169.0`, baixar as STL listadas em `bp_parts.json` para `bp/`,
  servir a pasta (`python3 -m http.server 8765`) e executar `node shot4.cjs`.
  Os arquivos STL (cerca de 176 MB) **não** estão versionados.

| Arquivo | Vista | Hotspots (% esquerda / topo) |
|---|---|---|
| `anat-frente.png` | Corpo inteiro | coração 53,1/27,5 · carótidas 50,2/20,9 · fígado 47,6/34,5 · rins 49,3/38,0 · bexiga 50,0/52,1 · cérebro 50,0/7,9 |
| `anat-cardio.png` | Tronco, foco cardio | coração 55,1/50,0 · carótidas 51,1/32,9 · pulmões 50,5/49,0 |
| `anat-abdome.png` | Tronco, foco abdome | fígado 47,8/35,1 · rins 50,8/41,6 · estômago 55,8/34,4 · bexiga 50,8/67,7 |

## Imagens de exame (`design/assets/imagens-exame/`)

- `tc-cranio-real`, `tc-torax-real`, `rm-abdome-real`, `us-doppler-real` e
  `mn-cintilografia-real` foram extraídas dos **arquivos DICOM de exemplo da
  biblioteca pydicom** (`pydicom/data/test_files`), com o script
  [`tools/imagens-dicom/extrair_exemplos.py`](../tools/imagens-dicom/extrair_exemplos.py).
  São só para ilustrar: **não há imagem de paciente da VidaCor**. Algumas têm
  texto do equipamento de origem gravado na imagem. Antes de qualquer uso público,
  conferir a licença de cada arquivo de exemplo.
- `us-eco` e `us-obst` são **sintéticas**, geradas proceduralmente só para o
  mockup.

## Marca (`design/assets/marca/`)

Recortes do logo a partir da imagem enviada pelo time
(`referencia-fachada-e-logo.jpg`). O original vetorial ainda precisa ser pedido
à clínica.

## Referências (`design/referencias/`)

Modelos de UI enviados pelo cliente (`ui-*`) e prints de feedback da rodada 2
(`feedback-*`). São de uso interno, como referência visual.

## Esquemas do padrão TISS (`crates/dominio-faturamento/xsd/`)

XSD oficiais da ANS, versão 4.03.00 (componente de comunicação de novembro de 2025,
`PadroTISSComunicao_202511.zip`, baixado de gov.br/ans em 08/11/2026, SHA-256 do zip
`db8640e1c3b87085892f54f838bfcea9934439ff365798c8428559f88c13d62d`). Copiados sem
alteração, só para validar o lote nos testes (DEC-2026-11-08-66). Padrão público da ANS
para os prestadores; termos de uso a confirmar.

## Descartado

O primeiro mapa 3D (manequim Xbot do repositório de exemplos do three.js) foi
substituído na rodada 3 e não está neste repositório.
