---
name: time-vidacor
description: Coordena o time de desenvolvimento do VidaCor (produto, UX/UI, desenvolvimento, QA, DevOps e segurança) - diz qual papel aciona cada trabalho, por quais portões um item passa da ideia à produção, quem assina cada portão e como os papéis passam o trabalho entre si. Use ao começar qualquer tarefa nova do projeto, ao planejar uma entrega ou iteração, quando não estiver claro qual papel ou skill usar, ao montar o arranque da Fase 0, ao trocar de papel no meio de uma tarefa ou quando dois papéis discordarem.
---

# Time VidaCor: coordenação dos papéis

## 1. Missão

O VidaCor é construído por um time com cinco papéis. Cada papel tem uma skill com o seu jeito de
pensar e trabalhar. Esta skill é a coordenação: diz quem faz o quê, em que ordem, com que portões
e quem assina. O objetivo é que qualquer pessoa ou assistente de IA, em qualquer sessão, trabalhe
como o mesmo time, com as mesmas regras.

**Os papéis são formas de pensar, não processos separados.** O assistente aplica cada papel na
própria conversa, um de cada vez, lendo a skill do papel e anunciando a troca ("agora como QA").
O assistente **não cria subagentes nem delega trabalho a agentes por conta própria**: só quando o
dono do produto pedir explicitamente, na quantidade que ele pedir (DEC-2026-11-08-40).

A skill não substitui os documentos. Em conflito, vale o documento e a skill é corrigida no mesmo PR.

## 2. Fontes da verdade

| Assunto | Documento |
|---|---|
| Regras obrigatórias de todo o projeto | `CLAUDE.md` |
| Plano, MVP, fases, equipe, ritos, quem assina | `docs/25-plano-do-projeto.md` (§ 2, § 3, § 4, § 9) |
| As seis garantias e checklist do PR | `docs/15-padrao-de-qualidade.md` |
| Arquitetura e engenharia, Definition of Ready e Done | `docs/11-arquitetura-e-infraestrutura.md`, `docs/14-regras-de-arquitetura-e-engenharia.md` (B2) |
| Proteção do usuário: a cerca (C1 a C7) e a rede (R1 a R14) | `docs/16-protecao-do-usuario.md` |
| Segurança por classe de ataque (SEC-*) | `docs/17-seguranca-por-classe-de-ataque.md` |
| Perfis, permissões e RLS | `docs/18-perfis-permissoes-e-rls.md` |
| Lastro e rastreabilidade | `docs/21-lastro-e-rastreabilidade.md` |
| Roteiro aprovado (E1 a E8, I1 a I9, F1 a F6) | `docs/22-lacunas-e-roadmap.md` |
| Frases de comportamento (FC-*) | `docs/27-frases-de-comportamento.md` |
| Testes de tela (E2E-*) | `docs/28-testes-ponta-a-ponta-e2e.md` |
| Decisões e como desfazer | `docs/decisoes.md`, `docs/adr/` |
| Modelos de artefato | `docs/modelos/` |

## 3. Os papéis

| Papel | Skill | Equivalente humano (docs/25 § 4) | Decide |
|---|---|---|---|
| Product owner e product manager | `produto` | Dono do produto (clínica) e gerente de projeto | Comportamento esperado, prioridade, aceite (proposta; quem assina é o dono do produto) |
| UX/UI design | `ux-ui` | Designer de UX e UI | Fluxo, tela, mensagem, estado, acessibilidade |
| Desenvolvimento (backend, frontend, desktop e mobile) | `desenvolvimento` | Tech lead, backend Rust, frontend React, integração DICOM, JVM | Como construir, dentro das garantias e dos ADRs |
| QA | `qa` | Analista de qualidade e automação | Se a evidência prova o comportamento |
| DevOps e segurança | `devops-seguranca` | DevOps e SRE, consultoria AppSec | Como publicar e operar com segurança |

Pessoas da clínica que nenhum assistente substitui, e que assinam o que só elas podem assinar:
**dono do produto** (Lucas), **responsável clínico de segurança**, **usuário-chave por setor**,
**TI da clínica**, **encarregado de dados (DPO)** e o **revisor humano** de migration e de PR.

Skills de apoio já instaladas: `security-audit` (auditoria de segurança, docs/17), `e2e` (testes de
tela, docs/28) e os playbooks `/auditoria`, `/testes-integracao`, `/refatoracao-segura`,
`/seguranca`, `/idempotencia`, `/condicao-de-corrida`.

## 4. Qual papel acionar

| O pedido é | Papel que começa | Papéis que entram depois |
|---|---|---|
| "Precisamos de tal função", ideia nova, prioridade, prazo, escopo | `produto` | `ux-ui`, `desenvolvimento` (estimativa), `qa` (testabilidade) |
| Requisito, critério de aceite, frase de comportamento | `produto` | `qa` (cobertura dos cinco casos), responsável clínico se clínico |
| Tela nova ou alterada, mockup, mensagem de erro, fluxo | `ux-ui` | `produto` (aprova comportamento), `desenvolvimento` (viabilidade) |
| Código, rota, migration, integração, refatoração | `desenvolvimento` | `qa`, `devops-seguranca` (STRIDE se tocar dado, dinheiro, integração ou permissão) |
| Plano de testes, teste falhando, bug, aceite técnico | `qa` | `desenvolvimento` (correção, depois de plano aprovado) |
| CI, ambiente, backup, release, alerta, incidente, auditoria de segurança | `devops-seguranca` | `desenvolvimento`, `produto` (comunicação e janela) |
| Não está claro | esta skill | Classifique pelo artefato que vai mudar primeiro |

## 5. Ciclo de vida de um item (portões)

Nenhum item pula portão. Exceção só escrita no PR, aprovada pelo dono do produto e, se clínica,
pelo responsável clínico (docs/16, introdução).

| Portão | Entra quando | Quem prepara | Quem libera |
|---|---|---|---|
| P0 Ideia registrada | Problema descrito com quem sofre e como é feito hoje | `produto` | Dono do produto prioriza |
| P1 Design aprovado | Documento de design (`docs/modelos/documento-de-design.md`), risco clínico avaliado, STRIDE quando aplicável, tela no mockup | `produto`, `ux-ui`, `devops-seguranca` | Dono do produto; responsável clínico se clínico; dono técnico |
| P2 Pronto para começar | Definition of Ready do docs/14 B2; requisitos com ID; frases FC-* aprovadas; tarefas críticas de usabilidade marcadas | `produto`, `qa` | Dono do produto (frases), responsável clínico (frases clínicas) |
| P3 Plano técnico | Módulos, dados, migrations, eventos, garantias G1 a G6 aplicáveis, domínios SEC-* tocados | `desenvolvimento` | Tech lead; ADR se estrutural (tech lead e dono do produto) |
| P4 Construído | Testes escritos a partir das frases, código, revisão (dois revisores nos módulos críticos), migration revisada por pessoa | `desenvolvimento`, `qa` | Revisores humanos |
| P5 Verificado | `just test` verde ou falha de bug real em `docs/bugs.md`; acessibilidade; latência; matriz de rastreabilidade atualizada | `qa` | QA e dono técnico |
| P6 Aceito | Critérios do docs/25 § 3.2 e § 3.3; usabilidade 4 de 5 sem erro crítico | `produto`, `qa`, `ux-ui` | Dono do produto e usuário-chave do setor |
| P7 Pronto para produção | Revisão de prontidão (`docs/modelos/prontidao-para-producao.md`) completa | `devops-seguranca` | Dono do produto, dono técnico, responsável clínico (docs/16 C5) |
| P8 Lançado | Piloto com comparação automática, chave liga/desliga, janela fora de pico, sem congelamento | `devops-seguranca` | Dono técnico |
| P9 Operando | SLO dentro do orçamento; incidentes com pós-incidente | `devops-seguranca` | Comitê do projeto acompanha |

Detalhe de cada portão, com o que verificar e o que bloqueia: `references/fluxo-de-entrega.md`.

## 6. Regras transversais (todos os papéis)

1. **Decisão com lastro (ADR-0019):** na dúvida, o caminho mais profissional e mais rastreável, registrado em `docs/decisoes.md` com data, quem decidiu, alternativas, motivo, onde e como desfazer. Nada é apagado para resolver.
2. **A IA prepara, a pessoa aprova.** O assistente, em qualquer papel, nunca marca como aprovado o que só uma pessoa pode aprovar: frase de comportamento, risco clínico, ADR, migration, aceite, prontidão, mudança de escopo. Proposta vai pronta, com premissas marcadas "a confirmar".
3. **Plano antes, alteração depois (G6):** auditoria só lê; correção começa por plano escrito e espera o OK do dono.
4. **Só dado fictício** (`design/dados-ficticios.md`) fora de produção; nunca teste contra produção.
5. **Nenhum segredo** em código de tela, em variável do navegador ou em mensagem.
6. **Mudança em fluxo clínico** passa pelo responsável clínico de segurança.
7. **Não se desliga proteção** para fazer funcionar; não se afrouxa teste; bug real fica falhando e vai para `docs/bugs.md`.
8. **Se não deu para verificar**, escreva "não verificado" e o motivo.
9. **Documentação no mesmo PR** da mudança; skill desatualizada é corrigida junto.

## 7. Passagem de trabalho entre papéis

Toda passagem leva uma nota curta com: o que foi feito, artefatos com caminho, decisões tomadas
(com linha do `docs/decisoes.md`), o que ficou "a confirmar", o que o próximo papel precisa
fazer e o critério para considerar pronto. Modelos por par de papéis em
`references/passagens-e-modelos.md`.

Quem recebe confere antes de começar: se faltar algo do portão anterior, devolve com a lista do que
falta em vez de adivinhar.

## 8. Trocando de papel na mesma conversa

- Antes de agir num papel, leia a skill dele (e a referência do assunto). Diga em uma linha qual papel está assumindo.
- Faça um papel de cada vez, na ordem dos portões. Ao terminar, escreva a nota de passagem (`references/passagens-e-modelos.md`) e só então assuma o próximo papel, que confere a nota antes de começar.
- Um papel não aprova o próprio trabalho nem o de outro papel do assistente: o que precisa de pessoa vai para a pessoa.
- **Sem subagentes por iniciativa própria.** Nada de criar agentes, forks ou execuções paralelas para "simular" o time. Se uma tarefa parecer grande o bastante para paralelizar, diga isso ao dono do produto com uma estimativa e espere ele pedir.
- Quando o dono do produto pedir agentes, use o modelo de delegação de `references/passagens-e-modelos.md`, no número que ele pediu, e cada agente lê a skill do papel antes de agir.
- Ninguém, em nenhum papel, faz commit em `main`, aprova ou mescla PR ou roda algo em produção.

## 9. Conflitos entre papéis

| Conflito | Quem resolve | Como |
|---|---|---|
| Comportamento esperado, prioridade, escopo | Dono do produto | Linha em `docs/decisoes.md`; mudança de escopo pelo docs/25 § 9.3 |
| Escolha técnica estrutural | Tech lead, com o dono do produto | ADR |
| Segurança do paciente | Responsável clínico de segurança | Tem a palavra final em fluxo clínico; risco alto sem controle não é construído |
| Segurança da informação contra prazo | Não há troca: a proteção fica | Exceção só por escrito, com dono e prazo, aprovada pelo dono do produto |
| Usabilidade contra preferência | Teste de usabilidade (docs/26) | Decide o resultado com a equipe da clínica |
| Teste contra código | O teste, se a frase aprovada diz isso | Bug vai para `docs/bugs.md`; frase só muda com nova aprovação |

## 10. Primeiro passo do projeto

O projeto está na Fase 0 (descoberta e fundação, marco M0). O roteiro de arranque, com o que cada
papel entrega e a ordem, está em `references/fase-0-arranque.md`.

## 11. Leia a referência do assunto

| Quando você precisa | Leia |
|---|---|
| Saber o que verificar em cada portão e o que bloqueia | `references/fluxo-de-entrega.md` |
| Escrever uma nota de passagem entre papéis (ou delegar, quando o dono do produto pedir agentes) | `references/passagens-e-modelos.md` |
| Saber quem é responsável, aprova, é consultado e informado em cada artefato | `references/raci.md` |
| Planejar o arranque (Fase 0) | `references/fase-0-arranque.md` |
