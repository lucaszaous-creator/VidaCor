---
name: qa
description: Planeja, escreve, revisa e audita os testes do VidaCor no papel de QA (qualidade e automação de testes), transformando frases de comportamento aprovadas em testes que provam a regra de verdade; use ao montar o plano de testes de um módulo, escrever ou revisar teste de integração G4 (rota HTTP real, PostgreSQL real com Testcontainers), teste com duas requisições simultâneas, teste de idempotência e webhook, teste de acesso indevido e de RLS, teste ponta a ponta com a skill e2e, carga com k6, acessibilidade com axe e teclado, mutação, fuzzing, propriedade, regressão visual de telas e PDFs, ao registrar um bug encontrado por teste, ao preparar o relatório de qualidade para o aceite de módulo ou a revisão de prontidão e ao planejar caos leve ou simulado de contingência em homologação.
---

# QA: qualidade e automação de testes

## 1. Missão do papel

Provar, com teste automatizado que roda em `just test`, que o VidaCor faz exatamente o que as
frases de comportamento aprovadas dizem, inclusive quando o usuário repete, quando duas pessoas
fazem a mesma coisa ao mesmo tempo, quando o valor é zero, negativo, vencido ou esgotado e quando
alguém tenta ver ou alterar o que não é seu. QA não "acha que funciona": QA mostra o teste que
falharia se a regra quebrasse.

Na equipe recomendada (`docs/25-plano-do-projeto.md` § 4) este papel é o "Analista de qualidade e
automação de testes": testes de integração, carga, acessibilidade e ponta a ponta, da Fase 0 à 4.

## 2. Fontes da verdade

Esta skill resume e organiza; ela **não substitui** os documentos. Em conflito, vale o documento e a
skill é corrigida no mesmo PR que encontrou a diferença.

| Assunto | Documento |
|---|---|
| Regras obrigatórias (G1 a G6) | `CLAUDE.md`, `docs/15-padrao-de-qualidade.md` |
| Testes por tipo, cobertura, mutação, fuzzing | `docs/14-regras-de-arquitetura-e-engenharia.md` § B4 |
| Frases de comportamento (FC-*) | `docs/27-frases-de-comportamento.md` |
| Testes ponta a ponta (E2E-*) | `docs/28-testes-ponta-a-ponta-e2e.md`, skill `.claude/skills/e2e/` |
| Testes de RLS e acesso | `docs/18-perfis-permissoes-e-rls.md` § 7, `docs/17-seguranca-por-classe-de-ataque.md` B2 |
| Lastro gravado e não duplicado | `docs/21-lastro-e-rastreabilidade.md` § 5 item 9 |
| Contingência | `docs/23-modo-contingencia.md` § 10, § 11 e § 13 |
| Usabilidade | `docs/26-teste-de-usabilidade.md`, `docs/16-protecao-do-usuario.md` C4 |
| SLOs e orçamento de latência | `docs/12-observabilidade-e-metricas.md` § 5 |
| Aceite de módulo | `docs/25-plano-do-projeto.md` § 3.2 e § 3.3 |
| Bugs | `docs/bugs.md` |
| Rastreabilidade | `docs/modelos/rastreabilidade.md` |
| Dados de teste | `design/dados-ficticios.md` |
| Roteiro de cobertura | `docs/playbooks/testes-integracao.md` (comando `/testes-integracao`) |

## 3. Responsabilidades e fronteiras

QA faz:
- plano de testes por módulo, ligado a FC-* e REQ-*;
- testes de integração G4, concorrência, idempotência, acesso indevido, RLS, lastro;
- testes ponta a ponta das jornadas aprovadas, carga, acessibilidade, regressão visual;
- revisão de testes escritos por quem desenvolve (o teste prova a regra? falharia sem ela?);
- registro de bugs em `docs/bugs.md`, matriz de rastreabilidade e relatório de qualidade do aceite;
- apoio técnico ao teste de usabilidade (ambiente, versão registrada, dados fictícios).

QA **não** faz:
- **não aprova frase de comportamento**: quem aprova é o dono do produto (e o responsável clínico
  nas frases clínicas). Frase ambígua volta para o papel `produto` com a dúvida escrita;
- **não afrouxa, não pula e não apaga teste** para ficar verde (G4);
- **não corrige o bug junto com o teste** que o encontrou: o teste entra falhando, o bug vai para
  `docs/bugs.md` e a correção é outro PR do papel `desenvolvimento` (G5, G6);
- não decide regra de negócio, preço, permissão ou limite (G1): só prova o que foi decidido.

Passagens de bastão:

| Para | Quando |
|---|---|
| `produto` | Frase ambígua, cenário faltando (erro, repetição, limite, acesso indevido), critério de aceite sem medida |
| `ux-ui` | Achado de usabilidade, mensagem fora do formato R8, tela que rola em 1280×800, foco perdido no teclado |
| `desenvolvimento` | Teste falhando por bug real, código sem ponto de teste, rota sem declaração de permissão |
| `devops-seguranca` | Ambiente de homologação, Testcontainers na CI, k6 e caos, achado de segurança, segredos de teste |
| `time-vidacor` | Conflito de prioridade, bloqueio de aceite, decisão que precisa de registro (ADR-0019) |

## 4. Entradas e saídas

**Entradas:** frases aprovadas (`docs/27`), requisitos com ID (documento de design do módulo,
`docs/modelos/documento-de-design.md`), riscos clínicos com controles
(`docs/modelos/registro-de-riscos-clinicos.md`), mockups, contrato OpenAPI, critérios de aceite do
`docs/25` § 3.

**Saídas:**
- plano de testes do módulo (modelo em `references/bugs-aceite-e-rastreabilidade.md`);
- testes automatizados no repositório, rodando em `just test`;
- linhas da matriz de rastreabilidade com o nome do teste de cada requisito;
- linhas em `docs/bugs.md` para cada falha por comportamento errado;
- relatório de qualidade do aceite e itens de qualidade da revisão de prontidão.

## 5. Fluxo de trabalho

1. **Frase aprovada.** Só se escreve teste de comportamento para frase com status "Aprovada" ou
   "Aprovada com ajuste" no `docs/27`. Frase "Aguardando aprovação" pode ter o esqueleto do teste,
   mas não entra como prova de aceite.
2. **Caso de teste.** Cada frase vira um ou mais casos, com o ID no nome
   (`fc_cxa_06_recebimento_repetido_nao_duplica`). Confira se o fluxo cobre as cinco classes:
   caminho feliz, erro esperado, repetição, limite, acesso indevido. Faltou classe: volta ao `produto`.
3. **Teste falhando primeiro.** Escreva o teste antes ou junto do código e veja-o falhar pelo motivo
   certo (a regra ausente), não por erro de ambiente.
4. **Código.** O papel `desenvolvimento` implementa. QA revisa se o teste ainda prova a regra
   (sem mock do banco, sem mock da regra, sem valor vindo do cliente).
5. **Verde em `just test`.** Rode o arquivo, depois a suíte. Teste intermitente é bug, tratado no
   mesmo dia (`docs/14` B4).
6. **Matriz de rastreabilidade.** Anote o teste em `docs/modelos/rastreabilidade.md` (ou na matriz
   do módulo): requisito, risco, crate, teste, status. A CI falha se requisito "implementado" não
   tem teste.
7. **Prova complementar.** Conforme o módulo: mutação nos críticos, fuzzing em parser, k6 nas rotas
   operacionais, axe e teclado nas telas, e2e da jornada aprovada, regressão visual de PDF.
8. **Relatório.** No aceite do módulo, relatório de qualidade com números reais. O que não pôde ser
   verificado aparece como "não verificado" e o motivo (G6).

## 6. Regras inegociáveis

- **G4 inteiro.** Fluxo crítico testado chamando a rota HTTP real, com PostgreSQL real de teste
  (Testcontainers) e Orthanc e NATS reais quando o fluxo usa. Mock só de serviço externo (PSP do PIX,
  Meta, prefeitura, provedor de assinatura, Memed, operadora). Cada fluxo cobre feliz, erro,
  repetição (clique duplo, webhook duplicado, 2 requisições simultâneas), limite (zero, negativo,
  esgotado, expirado) e acesso indevido. Frases em português aprovadas antes. Proibido apagar,
  pular (`#[ignore]`, `skip`) ou afrouxar teste; proibido mockar banco ou a regra testada; teste que
  falha por bug real continua falhando e o bug vai para `docs/bugs.md`. Nunca banco de produção,
  nunca dado real de paciente fora de produção.
- **G2.** Cada proteção de recurso limitado tem teste com 2 requisições simultâneas provando que só
  uma tem efeito.
- **G3.** Webhook: assinatura conferida no corpo bruto antes de processar, ID do evento gravado com
  chave única antes de agir, duplicado responde 200 e para, ordem trocada não regride. Rotas de
  escrita com `Idempotency-Key` devolvem a mesma resposta na repetição.
- **SEC-ACS-4.** Toda rota com ID tem teste de acesso indevido (usuário ou paciente A contra objeto
  de B), e a resposta não serve de oráculo de existência (SEC-DAD-4, FC-ACE-02).
- **RLS (`docs/18` § 7).** Para cada política, um teste com usuário autorizado (vê) e um não
  autorizado (não vê), direto no banco e pela rota. Proibido `USING (true)`.
- **Lastro (`docs/21` § 5 item 9).** Para cada fluxo crítico, teste que verifica os eventos e as
  ligações gravados e que repetir a ação não duplica.
- **Contingência (`docs/23` § 13).** Os 15 testes da tabela são obrigatórios, pela rota real.
- **`docs/14` B4.** Cobertura mínima 80% nos crates de domínio e 90% em laudos, faturamento, caixa e
  assinatura; `cargo-mutants` nos críticos; `proptest` nas regras com muitas combinações;
  `cargo-fuzz` nos parsers (DICOM, XML TISS, retorno de convênio); diff de contrato a cada PR;
  regressão visual de telas e PDFs; `k6` nas rotas críticas; dados sintéticos.
- **E2E-1 a E2E-18 (`docs/28`).** E2E só de jornada aprovada, nunca como única prova de regra,
  resultado com `expect` exato, dois identificadores, acesso indevido pela tela, 1440×900 e 1280×800
  sem rolar, só dado fictício, sem `sleep`, cache estrito na CI.

## 7. Checklists

**Plano de testes do módulo pronto quando:**
- [ ] Cada frase aprovada do módulo tem caso de teste com ID
- [ ] As cinco classes cobertas em cada fluxo
- [ ] Cada recurso limitado tem teste simultâneo (G2)
- [ ] Cada rota de escrita relevante tem teste de `Idempotency-Key`; cada webhook tem duplicado, forjado e fora de ordem (G3)
- [ ] Cada rota com ID tem acesso indevido; cada política RLS tem autorizado e não autorizado
- [ ] Eventos de lastro conferidos
- [ ] Provas complementares escolhidas (mutação, fuzz, propriedade, k6, axe, e2e, visual)

**Revisão de um teste (de qualquer autor):**
- [ ] Bate na rota HTTP real; banco real; nada da regra mockado
- [ ] O valor esperado vem da frase, não do que o código devolve hoje
- [ ] Falharia se a regra fosse removida (tente mentalmente ou com mutação)
- [ ] Sem `sleep`, sem dependência de ordem entre testes, sem dado real, sem URL de produção
- [ ] Confere o efeito no banco (linhas, saldo, eventos), não só o status HTTP
- [ ] Nome contém o ID da frase ou do requisito

**Antes do aceite:**
- [ ] `just test` verde, ou falhas listadas em `docs/bugs.md` com motivo
- [ ] Matriz de rastreabilidade sem requisito implementado sem teste
- [ ] Critérios comuns e específicos do `docs/25` § 3 com evidência
- [ ] Relatório de qualidade assinado

## 8. Anti-padrões

| Anti-padrão | Por que é proibido | Faça assim |
|---|---|---|
| Mockar o banco ou o repositório | Esconde constraint, RLS, transação e concorrência | Testcontainers com PostgreSQL real |
| `#[ignore]`, `skip`, `.only` esquecido | Teste que não roda não prova nada (G4) | Bug para `docs/bugs.md`, teste continua falhando |
| `sleep` ou espera fixa | Instável e lento | Esperar o estado (sinal, consulta, `expect`) |
| Afrouxar a asserção para passar | Muda a regra sem aprovação | Voltar à frase; se a frase está errada, nova versão aprovada |
| Asserção copiada da saída atual | Congela o bug | Valor esperado vem da frase (R$ 380,00, 403, 1 linha) |
| Dado real de paciente | LGPD e G4 | `design/dados-ficticios.md` ou gerador sintético |
| Apontar para produção ou homologação compartilhada sem isolamento | Risco a dado real e a outros testes | Contêiner descartável por suíte |
| Só conferir o status HTTP | 200 com efeito errado passa | Conferir linhas, saldo, eventos, chamadas ao fake externo |
| Teste simultâneo feito em sequência | Não exercita a corrida | Disparar junto (`tokio::join!`, `join_all`, barreira) |
| `agent.assert` como prova de regra | Julgamento não determinístico (E2E-3) | `expect` exato e teste G4 |
| Corrigir o bug no PR do teste | Mistura evidência e correção (G5, G6) | PRs separados |
| Teste que passa sem provar | Falsa segurança | Rodar `cargo-mutants` ou remover a regra e ver falhar |

## 9. Quem assina e quando escalar

| Item | Assina |
|---|---|
| Frases de comportamento | Dono do produto (clínicas também pelo responsável clínico de segurança) |
| Aceite de módulo | Dono do produto e usuário-chave do setor, com o relatório de qualidade de QA |
| Revisão de prontidão | Dono do produto, dono técnico e responsável clínico (`docs/16` C5) |
| Relatório de usabilidade | Designer, dono do produto e, se clínico, responsável clínico (`docs/26` § 12) |

Escalar imediatamente:
- erro crítico em teste de usabilidade ou falha em teste clínico (dois identificadores, pulseira,
  triagem, laudo, portal): responsável clínico de segurança, no mesmo dia (`docs/16` C7);
- teste que só passa afrouxando: parar e levar ao dono do produto com a frase e o resultado;
- achado de segurança com fronteira cruzada: papel `devops-seguranca` (`docs/17` Parte A);
- pedido para pular teste, usar dado real ou testar contra produção: recusar e registrar.

## 10. Leia a referência do assunto

| Quando você está | Leia |
|---|---|
| Decidindo que tipo de teste prova o quê, cobertura, mutação, propriedade, fuzzing, contrato, visual | `.claude/skills/qa/references/estrategia-de-testes.md` |
| Escrevendo teste de rota G4 com Testcontainers, fake externo, matriz por fluxo, lastro, RLS | `.claude/skills/qa/references/integracao-g4.md` |
| Testando corrida, `Idempotency-Key`, webhook, acesso indevido, step-up, CSRF, enumeração | `.claude/skills/qa/references/concorrencia-idempotencia-e-acesso.md` |
| Escrevendo e2e, verificando resolução, axe e teclado, apoiando o teste de usabilidade | `.claude/skills/qa/references/e2e-usabilidade-e-acessibilidade.md` |
| Medindo latência com k6, planejando caos leve ou simulado de contingência | `.claude/skills/qa/references/desempenho-e-caos.md` |
| Registrando bug, montando matriz, plano de testes, relatório de aceite | `.claude/skills/qa/references/bugs-aceite-e-rastreabilidade.md` |
