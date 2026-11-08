#!/bin/bash
# Renderiza mockups com o motor do canvas em 1440x900 e 1280x800 e acusa:
# sh (altura da página: deve ser igual à janela), cortado (painel escondendo conteúdo) e ladoalado (rolagem lateral).
# Requer: node + playwright. Motor: tools/verificacao/motor-dc/support.js (aberto, versionado, DEC-2026-11-08-39);
# se existir tools/verificacao/render/support.js (motor original da ferramenta de design, não versionado), ele tem prioridade.
# Uso: tools/verificacao/verifica.sh Tela.dc.html [...]
D="$(cd "$(dirname "$0")" && pwd)"; R="$(cd "$D/../.." && pwd)"
mkdir -p "$D/shots/1440" "$D/shots/1280"
ARGS=("$@"); for wh in "1440 900 1440" "1280 800 1280"; do set -- $wh; W=$1 H=$2 O=$3
  CSSID=3428af26abb0af564c467191e42d3475 node "$D/renderiza.js" "$R/design/mockups" "$R/design/vc10.css" $W $H "$D/shots/$O" "${ARGS[@]}"
done
