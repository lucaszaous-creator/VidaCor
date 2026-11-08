# Matriz de responsabilidade (RACI)

**R** responsável por fazer · **A** aprova e responde pelo resultado (uma só pessoa) · **C** consultado
antes · **I** informado depois.

Coluna "Clínica" reúne as pessoas da clínica; o papel exato está entre parênteses. A coluna "A" sempre
é uma pessoa: um agente nunca aprova. Base: `docs/25-plano-do-projeto.md` § 9.2 e `docs/16-protecao-do-usuario.md` C1.

| Artefato ou decisão | produto | ux-ui | desenvolvimento | qa | devops-seguranca | Clínica |
|---|---|---|---|---|---|---|
| Premissas (PREM-*) | R | I | C | I | C | A (dono do produto) |
| Priorização do backlog e da iteração | R | C | C | C | C | A (dono do produto) |
| Mudança de escopo, prazo ou orçamento | R | I | C | I | C | A (comitê do projeto) |
| Documento de design | R | R | C | C | C | A (dono do produto); C (responsável clínico se clínico) |
| Registro de riscos clínicos | R | C | C | C | I | A (responsável clínico) |
| Modelagem de ameaças (STRIDE) | C | I | R | I | R | I |
| Requisitos com ID | R | C | C | C | I | A (dono do produto) |
| Frases de comportamento (FC-*) | R | I | C | C | I | A (dono do produto); A (responsável clínico nas clínicas) |
| Mockup e especificação de tela | C | R | C | I | I | A (dono do produto) |
| Teste de usabilidade (rodada e relatório) | C | R | I | C | I | A (dono do produto); C (responsável clínico) |
| ADR | C | I | R | I | C | A (tech lead e dono do produto) |
| Código e PR | I | C | R | C | C | A (revisores humanos, CODEOWNERS) |
| Migration de banco | I | I | R | C | C | A (revisor humano, G6) |
| Plano e casos de teste | C | I | C | R | I | I |
| Registro de bug (`docs/bugs.md`) | I | I | C | R | I | I |
| Matriz de rastreabilidade | C | I | C | R | I | I |
| Pipeline de CI/CD | I | I | C | C | R | A (tech lead) |
| Ambientes e infraestrutura local | I | I | C | I | R | A (tech lead); C (TI da clínica) |
| Observabilidade, SLO e alertas | C | I | C | C | R | A (tech lead) |
| Auditoria de segurança | I | I | C | C | R | A (tech lead) |
| Aceite de módulo | R | C | C | R | I | A (dono do produto e usuário-chave) |
| Revisão de prontidão | C | I | C | C | R | A (dono do produto, dono técnico e responsável clínico) |
| Lançamento e volta atrás | I | I | C | C | R | A (dono técnico) |
| Comunicação de incidente à clínica | C | I | C | I | R | I (todos); C (responsável clínico se clínico) |
| Pós-incidente | C | I | R | C | R | A (dono técnico); A (responsável clínico se clínico) |
| Linha em `docs/decisoes.md` | R (comportamento) | R (tela) | R (técnica) | R (testes) | R (operação) | A (quem decidiu, registrado na linha) |

## Regras da matriz

1. Uma linha tem exatamente um "A" por tipo de decisão. Se a coluna Clínica tiver duas pessoas, cada uma aprova a sua parte (ex.: o responsável clínico aprova a parte clínica das frases).
2. Um agente pode ser "R" e preparar tudo, mas a aprovação ("A") fica registrada com nome e data de uma pessoa.
3. "C" é antes, não depois: consultar depois de fazer é "I".
4. Quando o "A" não responde, o item fica parado e aparece no relatório mensal ao comitê (`docs/25` § 9.4). Não se aprova por silêncio.
