# ADR-0020 · Testes ponta a ponta das telas com a skill e2e (TesterArmy)

- **Status:** proposto
- **Data:** 2026-11-08

## Contexto
O [docs/15](../15-padrao-de-qualidade.md) prevê "E2E de fumaça (Playwright) só nas jornadas aprovadas", sem definir ferramenta nem regras.
O dono do produto pediu a adoção da skill do projeto [tester-army/e2e](https://github.com/tester-army/e2e) (licença Apache-2.0):
um executor sobre Playwright que mistura passos exatos com passos de agente (modelo de linguagem), com repetição gravada sem chamar o modelo.

## Decisão
Instalar a skill em `.claude/skills/e2e/` (com `LICENSE`, `NOTICE` e commit de origem em `UPSTREAM_COMMIT`) e adotar o
[docs/28](../28-testes-ponta-a-ponta-e2e.md) com as regras `E2E-*`. O e2e é a camada de ponta a ponta das telas; a prova das
regras continua na integração (G4). Instalada só a skill `e2e`; as skills `create-verification-skill` e `migrate-to-e2e` do mesmo
repositório ficam de fora até haver necessidade.

## Alternativas
- Playwright puro: determinístico e sem modelo, mas sem a skill para o Claude Code e sem passos por objetivo. Continua possível: o e2e usa Playwright por baixo e aceita testes só com passos exatos.
- Não adotar ferramenta agora: adiaria a decisão para quando houver telas; o dono pediu a adoção.

## Consequências
- Passos de agente mandam a tela a um modelo: só com dado fictício, em ambiente de teste, preferindo modelo local (E2E-7 a E2E-9, SEC-IA-5).
- Julgamento do agente não é prova de regra: resultado importante sempre com `expect` exato (E2E-3).
- Dependência de terceiro: versão exata, lockfile e revisão a cada atualização da skill ou dos pacotes (E2E-17, SEC-SUP-1).
- Como desfazer: remover `.claude/skills/e2e/`, marcar este ADR como substituído e voltar o docs/15 a "Playwright".
