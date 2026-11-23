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
    mkdir -p apps/visualizador/dist
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    pnpm -r --if-present lint
    pnpm -r --if-present typecheck

# Verificação completa: testes do núcleo (com PostgreSQL real via Testcontainers) e das telas
test: lint
    cargo test --workspace || (just limpar-testes; exit 1)
    just limpar-testes
    pnpm -r --if-present test

# Visualizador VidaCor (docs/31) em desenvolvimento: tela com recarga e programa Tauri
visualizador:
    cd apps/visualizador && pnpm tauri dev

# Instalador do Visualizador (no Windows: gera .msi e .exe em target/release/bundle)
visualizador-instalador:
    cd apps/visualizador && pnpm tauri build

# Remove os contêineres e volumes criados pelos testes (Testcontainers); não toca no compose de dev
limpar-testes:
    docker ps -aq --filter label=org.testcontainers.managed-by=testcontainers | xargs -r docker rm -f -v > /dev/null

# Gera o contrato OpenAPI a partir do código e o cliente TypeScript
contrato:
    cargo run -q -p vidacor-api -- --openapi > packages/api-client/openapi.json
    pnpm --filter @vidacor/api-client gerar
