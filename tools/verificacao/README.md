# Verificação dos mockups

Ferramentas que conferem as telas de `design/mockups/*.dc.html` contra as regras do docs/05.

| Ferramenta | O que confere |
|---|---|
| `verifica.sh Tela.dc.html [...]` | A tela cabe na janela em 1440×900 e 1280×800: `sh` (altura da página igual à janela), `cortado` (painel escondendo conteúdo) e `ladoalado` (rolagem lateral). Fotos em `shots/` (fora do commit). |
| `sem-rolagem-lateral.js <pasta> design/vc10.css <largura>` | Rolagem lateral, sem motor (só o HTML cru). |
| `dados_ficticios.py` | Coerência dos pacientes fictícios entre telas. |

Requisitos: Node e Playwright com Chromium.

Para rodar em todas as telas:

```bash
tools/verificacao/verifica.sh $(cd design/mockups && ls *.dc.html)
```

## Motor de renderização (`motor-dc/support.js`)

Os `.dc.html` são modelos: têm `<sc-if>`, `<sc-for>`, `{{valor}}`, `<dc-import>` e uma classe
`Component extends DCLogic`. Para medir a tela de verdade, é preciso um motor que expanda os modelos.

- `motor-dc/support.js` é o **motor aberto e versionado** da VidaCor (DEC-2026-11-08-39). Ele cobre só o
  subconjunto do formato usado nos mockups e está descrito no cabeçalho do arquivo.
- O motor original da ferramenta de design nunca foi versionado e tem licença desconhecida. Se alguém tiver
  esse arquivo, pode colocá-lo em `render/support.js` (pasta fora do commit); `renderiza.js` dá prioridade a
  ele e informa qual motor usou na primeira linha da saída.
- Fidelidade do motor aberto ao original: **não verificada**, porque o original não está disponível para
  comparação.

## Limites conhecidos

- Imagens servidas como `/_blob/<id>` (logotipo, imagens de exame) não estão no repositório
  (`blobimg/` não existe). Elas respondem 404 e aparecem quebradas nas fotos. As medidas de telas com imagem
  sem tamanho fixo em CSS podem mudar quando as imagens existirem.
- Telas com tamanho próprio (apps 390×844, tablet 1180×820, telão 1280×720, impressos A4) acusam `sh`
  diferente da janela por definição (docs/05, exceções).
- `cortado` também aponta recortes de propósito, como molduras de imagem com `overflow: hidden` e zoom.
  Esses casos precisam de olho humano.
- `verifica.sh` lista os achados, mas não devolve código de erro.
