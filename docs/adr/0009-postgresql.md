# ADR-0009 · PostgreSQL como banco principal

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-06

## Contexto
Dados transacionais com integridade forte, guarda de 20 anos, auditoria e busca de pacientes.

## Decisão
PostgreSQL 17 com schemas por módulo, pg_trgm para busca, particionamento da auditoria, pgBackRest com PITR e réplica de streaming no servidor de contingência.

## Consequências
Banco maduro e conhecido pela equipe; compatível com Orthanc (plugin) e sqlx.
