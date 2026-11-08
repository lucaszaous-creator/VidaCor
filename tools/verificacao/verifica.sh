#!/bin/bash
# Verificador "a tela cabe na janela" (docs/05). Renderiza os mockups com o motor do canvas e SAI COM ERRO (1)
# se alguma tela reprovar; 2 se não conseguir rodar. Critério e exceções: verifica.js e excecoes.tsv (DEC-2026-11-08-42).
# Telas de sistema ($preview 1440x900): 1440x900 e 1280x800, página do tamanho exato da janela.
# Telas com tamanho próprio (apps, tablet, telão, A4, componentes): no próprio tamanho, sem passar dele.
# Em todas: sem painel cortando conteúdo (cortado), sem rolagem lateral (ladoalado), sem erro de script.
# Requer: node + playwright. Motor: tools/verificacao/motor-dc/support.js (aberto, versionado, DEC-2026-11-08-41);
# se existir tools/verificacao/render/support.js (motor original da ferramenta de design, não versionado), ele tem prioridade.
# Uso: tools/verificacao/verifica.sh [Tela.dc.html ...]   (sem argumentos: todas as telas de design/mockups)
D="$(cd "$(dirname "$0")" && pwd)"
exec node "$D/verifica.js" "$@"
