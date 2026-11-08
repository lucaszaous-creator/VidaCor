# Bugs, rastreabilidade e aceite

Fonte: `docs/bugs.md`, `docs/modelos/rastreabilidade.md`, `docs/25-plano-do-projeto.md` § 3.2 e
§ 3.3, `docs/modelos/prontidao-para-producao.md`, `docs/16` C5 e C7, `docs/26` § 12.

## 1. Fluxo de bug encontrado por teste

1. O teste falha. Confirme que a falha é do **sistema** (comportamento diferente da frase aprovada)
   e não do teste (ambiente, dado de preparo, asserção errada).
2. Se for do teste: corrija o teste sem mudar o comportamento esperado. Se o comportamento esperado
   parece errado, pare e leve ao dono do produto (frase nova versão, `docs/27` "Como aprovar" item 3).
3. Se for do sistema: o teste **continua falhando** e é commitado assim. Registre em `docs/bugs.md`.
4. A correção é outro PR, do papel `desenvolvimento`, que referencia o ID do bug. O teste que estava
   falhando passa a verde sem alteração na asserção.
5. Bug clínico (identificação, exame, laudo, portal, contingência) é comunicado ao responsável
   clínico de segurança no mesmo dia (`docs/16` C7).
6. Bug que chegou à produção ganha análise pós-incidente quando S1, S2 ou clínico, e a ação vira teste,
   regra, verificação de CI ou risco (`docs/16` C7 item 2).

Linha de `docs/bugs.md` (mesmas colunas do arquivo):

```markdown
| BUG-2026-0001 | Duas reservas simultâneas consomem a mesma guia; saldo fica -1 | `agenda::fc_agd_03_dois_agendamentos_simultaneos_com_a_mesma_guia` | 2026-11-20, PR do plano de testes da agenda | Aberto |
```

Gravidade do bug (alinhada a `docs/16` R14 e ao critério de usabilidade):

| Gravidade | Critério | Efeito no aceite |
|---|---|---|
| Crítico | Risco ao paciente, dinheiro errado, dado de saúde exposto, perda de dado | Bloqueia o módulo |
| Alto | Fluxo crítico não conclui ou conclui errado com contorno | Corrigido antes do aceite |
| Médio | Fluxo secundário com contorno | Planejado na próxima iteração com o dono do produto |
| Baixo | Incômodo | Lista de melhorias |

Formato do ID: proposta `BUG-AAAA-NNNN` (a confirmar; o arquivo hoje não define formato).

## 2. Matriz de rastreabilidade

Colunas do modelo (`docs/modelos/rastreabilidade.md`): Requisito, Descrição, Risco relacionado,
Módulo/crate, Teste(s), Status. Regras:
- todo `REQ-*` aponta para os testes que o comprovam; toda frase `FC-*` aparece no teste e na matriz;
- todo risco clínico `RC-*` com controle tem pelo menos um teste do controle;
- achado de usabilidade `ACH-USA-*` aponta para o requisito que cria ou corrige;
- status: Planejado, Em construção, Implementado, Aceito; "Implementado" sem teste reprova a CI.

Exemplo:

```markdown
| REQ-CXA-002 | Total do recebimento é recalculado pelo servidor e ignora o valor da tela | — | dominio-caixa | `caixa::fc_cxa_01_*`, `caixa::fc_cxa_02_*` | Implementado |
```

## 3. Modelo: plano de testes do módulo

```markdown
# Plano de testes: <módulo>  (versão, data, autor)

## Escopo
Frases: FC-... (status de cada uma). Requisitos: REQ-... Riscos: RC-...

## Matriz por fluxo
| Fluxo | Feliz | Erro | Repetição | Limite | Acesso indevido |
|---|---|---|---|---|---|

## Recursos limitados (G2) e proteção esperada
| Recurso | Proteção (índice único, atômica, FOR UPDATE) | Teste simultâneo |

## Idempotência (G3)
Rotas com Idempotency-Key, webhooks, chamadas externas com chave.

## Acesso e RLS
Rotas com ID e caso A contra B; políticas RLS e testes autorizado/não autorizado.

## Lastro
Eventos e ligações esperados por fluxo.

## Provas complementares
Mutação, propriedade, fuzzing, contrato, k6 (rotas e orçamento), axe, e2e (jornadas), visual.

## Ambiente e dados
Contêineres, fakes externos, relógio, dados fictícios usados.

## Fora do escopo e não verificado (com motivo)
```

## 4. Modelo: caso de teste ligado a frase e requisito

```markdown
| Campo | Valor |
|---|---|
| ID do teste | caixa::fc_cxa_06_recebimento_repetido_nao_duplica |
| Frase | FC-CXA-06 (status: Aprovada em <data>, por <dono do produto>) |
| Requisito / risco | REQ-CXA-006 / — |
| Classe | Repetição |
| Preparo | Carla (Caixa) com caixa aberto; Sônia Maria Castro, 03/02/1962; tabela v12, eco R$ 380,00 |
| Ação | POST /v1/caixa/recebimentos duas vezes com a mesma Idempotency-Key |
| Esperado | Respostas iguais; 1 recebimento; 1 cobrança no fake do PSP; 1 evento de lastro |
| Camada | Integração G4 (PostgreSQL e fake do PSP) |
```

## 5. Critérios de aceite (`docs/25` § 3.2)

Um módulo só é aceito com evidência de cada item:
1. documento de design aprovado;
2. frases aprovadas e testes correspondentes verdes em `just test`, cobrindo as cinco classes;
3. requisitos com ID ligados a teste na matriz;
4. riscos clínicos com controle implementado e testado; parecer clínico quando clínico;
5. usabilidade das tarefas críticas aprovada (4 de 5 sem erro e sem ajuda);
6. p95 abaixo de 150 ms nas telas operacionais em teste de carga;
7. WCAG 2.2 AA verificada (axe e teclado);
8. revisão de prontidão assinada.

Mais os critérios específicos do § 3.3 (por exemplo: agenda com duas reservas simultâneas e uma só
com efeito; caixa com webhook duplicado sem duplicar; portal com paciente A sem ver dado de B por
rota e RLS; faturamento com lote validado no XSD da ANS).

## 6. Modelo: relatório de qualidade do aceite

```markdown
# Relatório de qualidade: <módulo>, versão <commit>

## Resultado
Recomendação de QA: aceitar / não aceitar / aceitar com pendências aprovadas (listar)

## Números
- just test: N testes, N passam, N falham (todas as falhas em docs/bugs.md: IDs)
- Cobertura do domínio: N% (mínimo 80% ou 90%)
- Mutação: N mutantes, N sobreviventes (justificativas)
- Carga: p50/p95/p99 por rota contra o orçamento
- Acessibilidade: axe N violações sérias; teclado verificado em N telas
- E2E: jornadas N, verdes N, última execução com --no-cache em <data>

## Critérios do docs/25 § 3.2 e § 3.3
| Critério | Evidência | Situação |

## Rastreabilidade
Requisitos N, com teste N; riscos com controle testado N de N.

## Não verificado (motivo)

## Assinatura de QA e data
```

Este relatório alimenta a seção "Qualidade" da revisão de prontidão
(`docs/modelos/prontidao-para-producao.md`): `just test` verde ou falhas registradas, carga dentro do
orçamento, caos sem perda de dados.
