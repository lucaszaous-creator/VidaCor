# AUDITORIA — VidaCor

- **Data:** AAAA-MM-DD
- **Versão/commit auditado:** `<hash>`
- **Auditor:** <nome ou "assistente de IA, revisado por <nome>">
- **Escopo:** somente leitura. Nenhum arquivo alterado, nenhuma migration executada, nada instalado.

## Os 5 achados de maior risco
| # | Achado | Risco | Arquivo:linha | Garantia |
|---|---|---|---|---|
| 1 | | Alto | | G? |
| 2 | | | | |
| 3 | | | | |
| 4 | | | | |
| 5 | | | | |

## 1. Testes de integração (G4)
- Ferramenta: … · Banco de teste separado: sim/não · Comando: `just test`
- Resultado: **N passam, M falham** (ou "não verificado: motivo")

| Fluxo crítico | Teste na rota real | Só unitário com mock | Sem teste |
|---|---|---|---|
| | | | |

| Achado | Risco | Arquivo:linha | Correção sugerida |
|---|---|---|---|

## 2. Confiança no cliente (G1)
| Achado | Risco | Arquivo:linha | Correção sugerida |
|---|---|---|---|

Segredos encontrados:
| Variável/arquivo | Pública por design ou secreta | Já publicada? Rotacionar? |
|---|---|---|

Policies do portal (em português):
| Tabela | RLS | Policy | Quem pode o quê | Avaliação |
|---|---|---|---|---|

## 3. Condição de corrida (G2)
| Recurso | O que acontece com 2 requisições juntas | Proteção no banco hoje | Risco | Arquivo:linha | Correção sugerida |
|---|---|---|---|---|---|

## 4. Idempotência (G3)
| Webhook / chamada / rota | Corpo bruto verificado | ID único gravado antes | Chave de idempotência | Risco | Arquivo:linha | Correção sugerida |
|---|---|---|---|---|---|---|

## 5. Separação de responsabilidades (G5)
| Arquivo | Linhas | Mistura (tela / regra / dados) | Risco | Correção sugerida |
|---|---|---|---|---|

**Candidato nº 1 para refatorar:** …

## Itens não verificados
| Item | Motivo |
|---|---|

## Próximo passo
Rodar primeiro o playbook **/…**, porque …
