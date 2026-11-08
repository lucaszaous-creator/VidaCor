# ADR-0004 · Monólito modular em vez de microserviços

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-06

## Contexto
Equipe pequena, um servidor, muitos domínios que se relacionam (agenda, exames, laudos, faturamento).

## Decisão
Um binário Rust com um crate por bounded context, schema próprio no PostgreSQL por módulo, comunicação entre módulos por traits e eventos de domínio. Serviços separados só quando a tecnologia ou o local é outro.

## Consequências
Deploy e operação simples; fronteiras claras que permitem extrair um módulo no futuro.
