# ADR-0005 · Frontend em TypeScript + React, casca desktop em Tauri

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-06

## Contexto
As telas do mockup são densas, com tempo real (fila, imagens chegando) e exigência de acessibilidade. Balcão e salas usam periféricos (etiquetas, tablet de assinatura).

## Decisão
Aplicação web em TypeScript estrito, React, Vite e TanStack Query/Router, consumindo cliente gerado do OpenAPI. Design system VidaCor como pacote. Casca desktop opcional em Tauri (Rust) para periféricos e modo quiosque.

## Consequências
Ecossistema e acessibilidade maduros; mesmo código para web e desktop; contrato tipado ponta a ponta.
