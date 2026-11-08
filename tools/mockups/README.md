# Geradores de mockup (aposentados)

Os scripts `gerar_*.py` que montavam algumas telas foram **retirados em 08/11/2026** (decisão DEC-2026-11-08-05).
Motivo: as telas foram refeitas à mão no padrão "a tela cabe na janela" e sem "cara de IA" (docs/05). Rodar os
geradores recriaria o layout antigo por cima das telas aprovadas.

- Fonte da verdade das telas: `design/mockups/*.dc.html`.
- Para ver ou recuperar um gerador: `git show dde93c2:tools/mockups/<arquivo>` (última versão antes da retirada).
  Arquivos: `gerar_etiquetas.py`, `gerar_lacunas.py`, `gerar_lastro_e_guia.py`, `gerar_menus_contexto.py`.
- Verificação das telas: `tools/verificacao/verifica.sh` (largura, altura e corte) e
  `tools/verificacao/dados_ficticios.py` (coerência dos pacientes fictícios).
