# Aceite de módulo e governança

Fontes: `docs/25-plano-do-projeto.md` §3, §8 e §9; `docs/16-protecao-do-usuario.md` C5;
`docs/modelos/prontidao-para-producao.md`; `docs/26-teste-de-usabilidade.md` §11.

## 1. Critérios de aceite comuns a todo módulo (`docs/25` §3.2)

O PO confere cada item com evidência antes de pedir a assinatura:

| Nº | Critério | Evidência que o PO pede |
|---|---|---|
| 1 | Documento de design aprovado | Documento com as aprovações marcadas |
| 2 | Frases aprovadas e testes verdes em `just test`, cobrindo feliz, erro, repetição, limite e acesso indevido | Relatório do `just test` e tabela de status do `docs/27` |
| 3 | Requisitos com ID ligados a teste | Matriz de rastreabilidade sem lacuna |
| 4 | Riscos clínicos com controle implementado e testado; parecer clínico quando clínico | Registro de riscos e pacote do responsável clínico assinado |
| 5 | Usabilidade das tarefas críticas aprovada: 4 de 5 sem erro e sem ajuda, nenhum erro crítico | Relatório da rodada (`docs/26` §12) |
| 6 | Latência p95 abaixo de 150 ms nas telas operacionais | Relatório do teste de carga (k6) |
| 7 | WCAG 2.2 AA (axe e teclado) | Relatório do `qa` |
| 8 | Revisão de prontidão assinada antes da produção | `docs/modelos/prontidao-para-producao.md` preenchido |

Os critérios específicos por módulo estão no `docs/25` §3.3 (agenda, autorização, recepção,
caixa, contingência, sala, triagem, laudo, laudo estruturado, exame externo, portal,
faturamento, financeiro, estoque, migração). Copie a linha do módulo para o relatório de
aceite; não parafraseie.

Teste falhando por bug real **não** é removido para aceitar o módulo (G4): fica falhando, vai
para `docs/bugs.md`, e o aceite é recusado ou aceito com ressalva assinada pelo dono, com prazo.

## 2. Ritos (`docs/25` §9.1)

| Rito | Frequência | Papel do PO |
|---|---|---|
| Reunião diária | Diária, 15 min | Disponível para dúvidas de comportamento; registra impedimentos que dependem da clínica |
| Planejamento da iteração | Quinzenal | Traz o backlog priorizado e os itens prontos para começar (DoR) |
| Revisão da iteração com a clínica | Quinzenal | Conduz a demonstração em homologação com dados fictícios; colhe aceite ou ajuste |
| Comitê do projeto | Mensal | Apresenta o relatório mensal; leva decisões de escopo, prazo e custo |
| Revisão de risco clínico | Mensal e antes de cada módulo clínico | Leva os itens clínicos do ciclo e os achados de usabilidade clínicos |
| Revisão de mudança | A cada lançamento | Confere prontidão, janela e congelamento (C5) |
| Retrospectiva | Mensal | Participa; ações de melhoria com dono e prazo |

## 3. Quem assina o quê (`docs/25` §9.2)

| Artefato | Assina |
|---|---|
| Premissas | Dono do produto |
| Frases de comportamento | Dono do produto |
| Fluxo clínico, registro de riscos, alerta bloqueante novo | Responsável clínico de segurança |
| ADR | Tech lead e dono do produto |
| Migration | Revisor humano (G6) |
| Aceite de módulo | Dono do produto e usuário-chave do setor |
| Prontidão | Dono do produto, dono técnico e responsável clínico |
| Lotes de migração | Quem conferiu e responsável da clínica pelo dado |
| Virada e volta atrás | Comitê de virada |
| Escopo, prazo, orçamento | Comitê do projeto, com aprovação da direção |

## 4. Lançamento (`docs/16` C5), o que o PO confere

- Uso interno primeiro em homologação com dados fictícios.
- Canal piloto com comparação automática; liberação gradual por chave (perfil, máquina ou sala).
- Janela fora do pico, com dono técnico disponível e volta testada.
- **Congelamento:** nenhum lançamento em véspera de feriado, em fechamento de mês do faturamento ou com incidente aberto.
- Mudança visível avisada antes à equipe (R9) e treinamento feito para função crítica (R12).

## 5. Riscos do projeto (`docs/25` §8)

O PO, como gerente de projeto, mantém a tabela de riscos viva: revisa no comitê mensal,
atualiza probabilidade e impacto, e registra em `docs/decisoes.md` quando a mitigação muda.
Riscos que mais dependem do PO: premissas não confirmadas, disponibilidade da equipe da
clínica, escopo que cresce, fornecedores atuais que não entregam dados.

## 6. Modelos

```markdown
## Revisão da iteração AAAA-NN com a clínica
- Data: DD/MM/AAAA | Participantes: <nomes e papéis>
- Versão mostrada: <commit> em homologação | Dados: somente fictícios
### Pauta
1. Itens concluídos na iteração (demonstração)
2. Itens não concluídos e motivo
3. Achados de usabilidade e bugs abertos relevantes
4. Decisões pedidas à clínica
5. Próxima iteração (proposta)
### Ata
| Item | Resultado (aceito / ajuste / recusado) | Ajuste pedido | Dono | Prazo |
|---|---|---|---|---|
Decisões registradas: DEC-...
```

```markdown
## Relatório mensal ao comitê, <mês/ano>
1. Entregas do mês e marcos (previsto x realizado)
2. Custo realizado x previsto (faixas e premissas usadas)
3. Riscos do projeto: mudanças de nível e mitigação
4. Qualidade: testes (verdes/falhando com bug registrado), defeitos por gravidade, latência p95 das rotas críticas
5. Usabilidade: rodadas feitas, tarefas aprovadas e reprovadas, SUS por perfil
6. Decisões tomadas no mês (lista de DEC-*)
7. Decisões pedidas ao comitê (com recomendação)
8. Premissas confirmadas e pendentes
```

```markdown
## Relatório de aceite do módulo <nome>
- Versão: <commit/tag> | Data: DD/MM/AAAA
| Critério (docs/25 §3.2 e §3.3) | Evidência | Situação |
|---|---|---|
| 1 Documento de design aprovado | <link> | Atendido / Não atendido |
...
- Ressalvas (com prazo e dono): <lista ou "nenhuma">
- Assinaturas: dono do produto [ ] usuário-chave do setor [ ]
```
