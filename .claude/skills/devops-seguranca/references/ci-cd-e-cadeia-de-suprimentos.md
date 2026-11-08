# CI/CD e cadeia de suprimentos

Fontes: `docs/11-arquitetura-e-infraestrutura.md` §9.3 e §9.4, `docs/14-regras-de-arquitetura-e-engenharia.md`
B1, B3, B4 e B5, `docs/17-seguranca-por-classe-de-ataque.md` B10, `docs/28-testes-ponta-a-ponta-e2e.md`.

## 1. Etapas da esteira

A ordem é fixa (docs/11 §9.4). Uma etapa vermelha para a esteira; nada é publicado com etapa pulada.

| Etapa | O que roda | Bloqueia |
|---|---|---|
| 1. Lint e formato | `rustfmt --check`, `clippy -D warnings`, ESLint, Prettier, `tsc --noEmit` com `strict`, `ruff`, `mypy --strict`, `ktlint`, `detekt`, `squawk` nas migrações | Sim |
| 2. Segurança estática | Varredura de segredos, Semgrep e CodeQL, lint de SQL montado com `format!` (SEC-INJ-1), lint de `dangerouslySetInnerHTML` (SEC-CLI-1), `unsafe` fora do crate de FFI (SEC-MEM-1) | Sim |
| 3. Dependências e licenças | `cargo-deny` (fontes, licenças, banidos), `cargo-audit`, `pnpm audit`, `pip-audit` | Sim |
| 4. Testes | Unidade; integração com Testcontainers (PostgreSQL, Orthanc, NATS); teste de arquitetura (nenhum módulo lê tabela de outro); lint de migração (sem `UPDATE`/`DELETE` em tabela clínica, sem `USING (true)`); cobertura mínima (80% domínio, 90% laudos, faturamento, caixa e assinatura) | Sim |
| 5. Build | Binários Rust, aplicação web, Desktop Tauri (MSI), serviços Python e Kotlin | Sim |
| 6. Contrato | Diff do OpenAPI e dos schemas de evento; mudança incompatível só em nova versão (docs/14 A4) | Sim |
| 7. Ponta a ponta e acessibilidade | E2E nas jornadas aprovadas com cache estrito; `axe`; `k6` nas rotas críticas com orçamento p95 | Sim |
| 8. Imagens | Contêineres construídos uma vez, assinados, com SBOM e proveniência | Sim |
| 9. Publicação | Canal `piloto` e depois `estavel` no repositório de versões; o servidor da clínica **puxa** | Só com prontidão |

Regras gerais:
- `just` é o executor único também na CI (`just lint`, `just test`). A CI chama as mesmas receitas que a
  pessoa roda localmente.
- Teste instável é bug: corrigido ou isolado no mesmo dia, registrado em `docs/bugs.md`. Proibido
  `#[ignore]`, `skip` ou retentativa automática para esconder falha (G4).
- Testes nunca apontam para banco de produção nem usam dado real (G4).

## 2. Proteção do repositório

- `main` protegida: merge só por PR com CI verde e revisão; 2 revisores via `CODEOWNERS` em laudos,
  assinatura, faturamento, caixa, auditoria, identidade e exames/DICOM (docs/14 B1, docs/16 C1).
- Proibido push direto e force-push em `main`.
- Commits no padrão Conventional Commits; versão SemVer; changelog gerado.
- `CODEOWNERS` também cobre `.github/`, `infra/` e `crates/comum/` (telemetria e auditoria): mudança de
  esteira e de infraestrutura passa pelo papel de DevOps e segurança.

## 3. Regras de cadeia de suprimentos

| ID | Regra | Como verificar |
|---|---|---|
| SEC-SUP-1 | Lockfiles versionados (`Cargo.lock`, `pnpm-lock.yaml`, lock do Python, Gradle lock); fontes permitidas no `deny.toml` | `cargo deny check`; PR sem lockfile atualizado falha |
| SEC-SUP-2 | Actions fixadas por SHA completo; sem `pull_request_target` executando código do PR; segredos só em jobs que não rodam código de terceiros | Lint de workflow (por exemplo `zizmor` ou `actionlint`, a confirmar a ferramenta) |
| SEC-SUP-3 | `permissions` mínimas por job; cache por branch confiável | Revisão do workflow; `permissions: {}` no topo |
| SEC-SUP-4 | O que é promovido é o que foi construído, pelo hash; assinatura e proveniência | Promoção por digest da imagem, nunca por tag |
| SEC-SUP-5 | Metadados de atualização assinados, sem *rollback* para versão vulnerável | Updater do Tauri e agente do servidor verificam assinatura e versão mínima |

Dependências novas:
1. Justificativa no PR (o que resolve, alternativas, manutenção ativa, licença).
2. Licença compatível com uso comercial; atribuição em `THIRD_PARTY_NOTICES.md`.
3. Versão exata e lockfile. Pacotes `e2e` e `@e2e-dev/*` com versão exata (docs/28 E2E-17).
4. Skills de terceiros em `.claude/skills/` só por cópia de um commit de origem, com SHA em
   `UPSTREAM_COMMIT` e revisão da diferença.

## 4. Exemplo de job (esboço)

Valores entre `<...>` são placeholders: o SHA real é obtido na página de releases da action e conferido
na revisão. Não copie SHA de memória.

```yaml
name: ci
on:
  pull_request:
  push:
    branches: [main]

permissions: {}            # nada por padrão (SEC-SUP-3)

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  testes:
    runs-on: ubuntu-24.04
    permissions:
      contents: read       # só o necessário para este job
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@<sha-completo> # vX.Y.Z
        with:
          persist-credentials: false
      - uses: <dono>/<action-de-toolchain>@<sha-completo> # vX.Y.Z
      - name: Lint
        run: just lint
      - name: Testes (Testcontainers, sem banco de produção)
        run: just test
```

Pontos de revisão do exemplo:
- `on: pull_request` (nunca `pull_request_target` com checkout do código do PR).
- Nenhum segredo neste job: ele roda código do PR.
- O job de publicação é separado, roda só em `main`, recebe `id-token: write` apenas se usar assinatura
  sem chave (*keyless*) e `packages: write` apenas se publicar imagem.

## 5. Imagens, SBOM e assinatura

- Uma imagem por serviço, construída uma única vez por versão; promovida de `piloto` para `estavel`
  pelo **digest**.
- SBOM por versão (formato SPDX ou CycloneDX, a confirmar) publicado junto da imagem e do MSI.
- Assinatura das imagens e da proveniência (por exemplo cosign; ferramenta **a confirmar** em ADR de
  publicação). O servidor da clínica recusa imagem sem assinatura válida.
- MSI do Desktop assinado com certificado de assinatura de código (docs/13 §6); o updater do Tauri usa
  chave própria de atualização, guardada no cofre e separada do certificado de código.

## 6. Implantação puxada pela clínica

O servidor local **não aceita conexão de entrada** da internet (docs/11 §9.4, docs/13 §5):
1. A CI publica a versão assinada no repositório de versões.
2. O agente de atualização do servidor da clínica baixa por conexão de saída e verifica assinatura,
   hash e versão mínima.
3. A instalação acontece na janela de manutenção, com backup antes e volta em um comando.
4. O servidor vira o repositório local das estações; as estações nunca falam com a internet para
   atualizar.

## 7. Checklist de revisão de workflow

- [ ] `permissions: {}` no topo e permissões por job
- [ ] Toda action por SHA completo com comentário da versão
- [ ] Nenhum `pull_request_target`, `workflow_run` com artefato não confiável sem verificação, ou script
      que interpola `${{ github.event.* }}` direto no shell
- [ ] Segredos só em jobs de `main` que não executam código do PR
- [ ] `timeout-minutes` em todo job
- [ ] Artefatos de teste (`.e2e/`, relatórios) com retenção curta e sem dado real
- [ ] Publicação por digest, com SBOM e assinatura
