# Playbooks de auditoria e correção

Roteiros padronizados para auditar e corrigir o VidaCor. Cada um também existe como comando do Claude Code em `.claude/commands/` (por exemplo, `/auditoria`).

| Ordem sugerida | Playbook | Comando | Garantia |
|---|---|---|---|
| 1 | [Auditoria geral (somente leitura)](auditoria.md) | `/auditoria` | G1–G5 (leitura) |
| 2 | [Testes de integração dos fluxos críticos](testes-integracao.md) | `/testes-integracao` | G4 |
| 3 | [Auditoria de segurança: cliente, segredos e banco](seguranca.md) | `/seguranca` | G1 |
| 4 | [Idempotência de webhooks e pagamentos](idempotencia.md) | `/idempotencia` | G3 |
| 5 | [Condições de corrida](condicao-de-corrida.md) | `/condicao-de-corrida` | G2 |
| 6 | [Refatoração segura em camadas](refatoracao-segura.md) | `/refatoracao-segura` | G5 |
| — | Auditoria completa de segurança (Cloudflare) | `/security-audit` | G1 + docs/17 |
| — | Testes ponta a ponta das telas (TesterArmy) | skill `e2e` | G4 + docs/28 |

Fluxo recomendado: **auditoria** (só leitura) → AUDITORIA.md aponta o primeiro playbook de correção → plano → OK do dono → correção em passos → `just test`.

Modelo do relatório: [modelo-auditoria.md](modelo-auditoria.md).
