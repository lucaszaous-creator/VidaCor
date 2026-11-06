# ADR-0002 · Rust no núcleo do sistema

- **Status:** proposto
- **Data:** 2026-11-06

## Contexto
Servidor local, exigência de baixa latência no balcão, alta confiabilidade e operação contínua. A equipe quer linguagens robustas e profissionais.

## Decisão
O núcleo (API, domínios, integrações HTTP, gateway DICOM, geração de documentos) é escrito em Rust com axum, tokio, sqlx, utoipa e tracing. Documentos são gerados com Typst.

## Consequências
Desempenho previsível e baixo consumo de memória; segurança de memória e de concorrência; curva de aprendizado maior, mitigada por esqueleto de referência e convenções. Perde-se o reaproveitamento direto da suíte Clinica (.NET), cujas regras e testes passam a ser especificação.
