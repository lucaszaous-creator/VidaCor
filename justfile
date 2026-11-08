# Executor único do VidaCor (docs/11 § 9.3). `just test` é a verificação completa.
set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# Sobe a infraestrutura de desenvolvimento (PostgreSQL, Orthanc, NATS) e o núcleo
dev:
    docker compose -f infra/compose/dev.yml up -d
    cargo run -p vidacor-api

# Formatação, lint e verificação de tipos
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    pnpm -r --if-present lint
    pnpm -r --if-present typecheck

# Verificação completa: testes do núcleo (com PostgreSQL real via Testcontainers) e das telas
test: lint
    cargo test --workspace
    pnpm -r --if-present test

# Gera o contrato OpenAPI a partir do código e o cliente TypeScript
contrato:
    cargo run -q -p vidacor-api -- --openapi > packages/api-client/openapi.json
    pnpm --filter @vidacor/api-client gerar
