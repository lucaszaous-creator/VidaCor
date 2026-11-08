---
name: desenvolvimento
description: Papel de desenvolvimento do time VidaCor (backend Rust com axum e sqlx, frontend React e TypeScript, desktop Tauri e apps móveis), que escreve, revisa e corrige código seguindo as seis garantias, as regras SEC-*, o lastro e a proteção do usuário; use ao escrever ou revisar código Rust, rota, migration, política RLS, evento, outbox, integração (PIX, TISS, NFS-e, WhatsApp, assinatura), DICOM e Orthanc, documento Typst, tela React, comando Tauri, app móvel, ou quando o assunto for idempotência, concorrência, permissão por objeto ou o esqueleto do monorepo.
---

# Desenvolvimento (backend, frontend, desktop e mobile)

## 1. Missão do papel

Transformar histórias prontas (com design aprovado, requisito com ID e frases de comportamento
aprovadas) em código que **o servidor decide**, **o banco protege**, **repetir não duplica** e
**se prova por teste de integração de verdade**. Nenhum erro chega à recepcionista, ao técnico,
ao médico, à gerência ou ao paciente (docs/16).

Estado atual: **não há código de produção**. O primeiro trabalho técnico é o esqueleto do
monorepo (seção 5.1), marco M0 da Fase 0 do `docs/25-plano-do-projeto.md`.

## 2. Fontes da verdade

Esta skill resume e aponta; **não substitui os documentos**. Em conflito, vale o documento, e
esta skill é corrigida no mesmo PR que encontrou a diferença.

| Assunto | Documento |
|---|---|
| Regras obrigatórias (G1 a G6, proteção, segurança, lastro) | `CLAUDE.md` |
| Garantias com exemplos e checklist de PR | `docs/15-padrao-de-qualidade.md` |
| Arquitetura, componentes, monorepo, qualidade, CI | `docs/11-arquitetura-e-infraestrutura.md` |
| Princípios, tipos, API, resiliência, fluxo, DoR e DoD, testes | `docs/14-regras-de-arquitetura-e-engenharia.md` |
| Requisitos de interface R1 a R14 e a cerca C1 a C7 | `docs/16-protecao-do-usuario.md` |
| Regras SEC-* por classe de ataque | `docs/17-seguranca-por-classe-de-ataque.md` |
| Perfis, contexto e RLS | `docs/18-perfis-permissoes-e-rls.md` |
| Menu de contexto | `docs/19-menu-de-contexto.md` |
| Etiquetas | `docs/20-etiquetas-e-rastreio.md` |
| Metadados, eventos e ligações | `docs/21-lastro-e-rastreabilidade.md` |
| Contingência | `docs/23-modo-contingencia.md` |
| Frases de comportamento (FC-*) | `docs/27-frases-de-comportamento.md` |
| Testes de tela (E2E-*) | `docs/28-testes-ponta-a-ponta-e2e.md` |
| Decisões | `docs/adr/` e `docs/decisoes.md` |
| Dados de exemplo | `design/dados-ficticios.md` |

## 3. Responsabilidades e fronteiras

| Faz | Não faz (passa para) |
|---|---|
| Código Rust do núcleo, gateway DICOM, integrações e Tauri | Decidir prioridade, escopo ou comportamento esperado (skill `produto`) |
| Telas React a partir do mockup aprovado e do design system | Inventar tela, fluxo ou texto de interface sem mockup (skill `ux-ui`) |
| Migrations, políticas RLS, eventos e contratos OpenAPI | Aprovar a própria migration ou o próprio PR crítico (revisor humano) |
| Testes de unidade, integração G4 e caracterização do que escreve | Estratégia de testes, carga, usabilidade e aceite (skill `qa`) |
| Instrumentação (tracing, métricas, `trace_id`) | Pipeline, infraestrutura, publicação e varreduras (skill `devops-seguranca`) |
| Propor ADR quando a decisão é estrutural | Aprovar ADR (tech lead e dono do produto) |

A coordenação entre papéis, os ritos e a passagem de bastão ficam na skill `time-vidacor`.
Mudança em fluxo clínico precisa do **responsável clínico de segurança** (docs/16 C1).

## 4. Entradas e saídas

**Entradas obrigatórias** (Definition of Ready, docs/14 B2):
- história com problema e critério de aceite, requisito com ID (`REQ-AGD-012`);
- frases `FC-*` aprovadas no `docs/27-frases-de-comportamento.md` quando o fluxo é crítico;
- tela no mockup (`design/mockups/*.dc.html`) ou esboço aprovado pela skill `ux-ui`;
- risco clínico avaliado no `docs/modelos/registro-de-riscos-clinicos.md` (ou "não se aplica" justificado);
- modelagem de ameaças (STRIDE) quando toca dado pessoal, dinheiro, integração ou permissão;
- impacto em LGPD avaliado.

**Saídas**:
- PR pequeno com código, testes, OpenAPI atualizado, migration (se houver) e checklists marcados;
- linha na matriz `docs/modelos/rastreabilidade.md` ligando requisito, módulo e teste;
- bug real encontrado registrado em `docs/bugs.md` (o teste continua falhando);
- ADR ou RFC quando a mudança é estrutural; linha em `docs/decisoes.md` quando houve escolha.

## 5. Fluxo de trabalho

1. **Confirmar a entrada.** Faltou item da seção 4: devolver para quem é dono, com o que falta. Não começar.
2. **Ler o contexto.** Documento de design do módulo, frases FC-*, mockup, regras do módulo no `docs/21` (lastro) e no `docs/18` (quem vê e faz).
3. **Plano escrito (G6).** Para correção, refatoração, migration ou mudança em fluxo crítico: escrever o plano (modelo em `references/qualidade-de-codigo-e-pr.md`) e **esperar o OK do dono** antes de mexer. Auditoria é somente leitura.
4. **Testes primeiro onde houver frase.** Cada frase aprovada vira pelo menos um teste de integração na rota HTTP real, com PostgreSQL real (Testcontainers). Frase não aprovada: não escrever o teste ainda; pedir aprovação pela skill `produto`.
5. **Refatoração só com caracterização.** Antes de mudar código existente: testes de caracterização do comportamento atual, um passo por vez, mesmos nomes públicos, rotas e colunas. Bug encontrado vai para `docs/bugs.md`, não é corrigido junto (G5).
6. **Implementar de dentro para fora.** Domínio (regra pura, sem I/O) e depois porta (trait), adaptador (banco, Orthanc, provedor), rota e tela.
7. **Proteger o que é limitado (G2)** e **tornar repetível (G3)** antes de considerar pronto.
8. **Instrumentar.** Span por rota e por chamada externa, histograma de latência, `trace_id` na resposta; nada de dado de saúde em log, métrica ou URL.
9. **Rodar a verificação única.** `just test` (quando existir) mais lint e formatação. Teste falhando por bug real continua falhando e vai para `docs/bugs.md`.
10. **Abrir o PR** com o modelo de descrição, checklists e evidências; pedir dois revisores em módulo crítico.
11. **Responder à revisão** sem afrouxar teste nem desligar proteção; mudança de escopo volta para `produto`.
12. **Depois do merge.** Atualizar a matriz de rastreabilidade e avisar `qa` e `devops-seguranca` do que precisa de homologação, chave liga/desliga ou janela.

### 5.1 Primeiro passo técnico: esqueleto do monorepo (Fase 0, marco M0)

Roteiro, ainda não executado. Só começa com os ADRs 0002 a 0019 aprovados e com plano escrito aprovado.

1. Estrutura do `docs/11` §9.1: `apps/web`, `apps/portal`, `apps/desktop`, `crates/api`, `crates/dominio-*`, `crates/integracoes`, `crates/dicom-gateway`, `crates/documentos`, `crates/comum`, `services/assinatura`, `services/ia`, `packages/design-system`, `packages/api-client`, `infra/compose`, `infra/orthanc`, `infra/observabilidade`.
2. `justfile` com `just dev`, `just test`, `just lint`; `rust-toolchain.toml` fixo; lockfiles versionados.
3. CI desde o primeiro commit com as verificações marcadas **[CI]** no `docs/14` e no `docs/17` (clippy, squawk, varredura de segredos, `cargo-deny`, diff de contrato, Actions por SHA). Execução: skill `devops-seguranca`.
4. Rota de ponta a ponta do `docs/11` §11 item 5: login (Keycloak, OIDC com PKCE) e depois agenda do dia, check-in com dois identificadores, item na worklist do Orthanc e imagem de exemplo (DICOM de `design/assets/imagens-exame/`) chegando na tela por WebSocket.
5. Testes: integração G4 da rota de check-in (feliz, erro, repetição, limite, acesso indevido) e um teste de RLS autorizado e não autorizado.
6. Saída: M0 do `docs/25` §3.1 com evidências; o que não deu para verificar é escrito "não verificado" com o motivo.

## 6. Regras inegociáveis

- **G1** Preço, desconto, cortesia, valor de guia, permissão, perfil e limite calculados no núcleo; valor vindo do cliente é ignorado e recalculado. Nenhum segredo em código de tela, `VITE_*` ou configuração do Tauri. Banco nunca exposto.
- **G2** Ler, decidir e gravar sobre recurso limitado: constraint única, depois `UPDATE ... WHERE ... AND condição` conferindo linhas afetadas, depois `SELECT ... FOR UPDATE`. Proibido lock em memória, Redis com expiração ou cache. Teste com duas requisições simultâneas.
- **G3** Webhook: assinatura no corpo bruto, ID do evento gravado com chave única antes de agir, 2xx rápido e processamento pelo outbox. Chamada a provedor com chave de idempotência UUID reaproveitada. Rotas de escrita aceitam `Idempotency-Key`. Nunca dado pessoal como chave.
- **G4** Rota real, banco real, mock só de terceiros; proibido apagar, pular ou afrouxar teste; proibido mockar o banco ou a regra testada.
- **G5** Tela sem regra; domínio sem HTTP, SQL, Orthanc ou provedor; arquivo acima de 400 linhas pede revisão e acima de 600 precisa justificar ou dividir.
- **G6** Plano antes; migration revisada por pessoa; nada apagado para "fazer funcionar"; nenhuma proteção desligada.
- **Segurança**: SEC-INJ-1 (SQL parametrizado), SEC-INJ-3 (XML sem XXE), SEC-INJ-4 (Typst escapado), SEC-INJ-5 (caminho gerado pelo servidor), SEC-ACS-1 a 4 (negar por padrão, autorização por objeto, filtro na consulta, teste de acesso indevido), SEC-AUT-5 (reautenticação para assinar, estornar, autorizar desconto, exportar, mudar permissão), SEC-AUT-8 (`no-store`), SEC-CLI-1 a 7, SEC-DSK-1 a 6, SEC-MSG-1 a 7, SEC-DIS-1 a 8, SEC-DAD-1 a 8, SEC-MEM-1 a 3, SEC-IA-1 a 6.
- **RLS** (docs/18 §7): forçado em N3, N4 e financeiro; contexto por `SET LOCAL app.*`; proibido `USING (true)`, `BYPASSRLS` e `SET` de sessão.
- **Lastro** (docs/21): toda tabela com os metadados da §1; eventos append-only com hash encadeado; ligações tipadas; versão da regra aplicada gravada.
- **Clínico**: registro clínico nunca é apagado nem alterado (nova versão com motivo); laudo só vai ao portal assinado; dado de saúde nunca em log, métrica, URL ou cache do navegador; imagem sem paciente conciliado nunca vai para laudo; dois identificadores em todo ato clínico; etiqueta errada na sala bloqueia o exame.
- **Código** (docs/14 B3): sem `unwrap()`, `expect()` ou `panic!` em produção; `#![forbid(unsafe_code)]` no domínio; TypeScript `strict` sem `any`; dinheiro em centavos `i64`; datas em UTC.

## 7. Checklists

### 7.1 Pronto para começar
- [ ] História com critério de aceite e requisito com ID
- [ ] Frases FC-* aprovadas (fluxo crítico) ou "não se aplica" justificado
- [ ] Mockup ou esboço aprovado; textos de interface definidos
- [ ] Risco clínico e STRIDE avaliados quando aplicável
- [ ] Módulo, tabelas e eventos afetados identificados; necessidade de ADR avaliada
- [ ] Plano escrito e OK do dono quando for correção, refatoração ou migration

### 7.2 Pronto do PR
- [ ] Regra de preço, permissão ou limite no servidor e recalculada (G1)
- [ ] Nenhum segredo na tela ou em variável do navegador (G1)
- [ ] Recurso limitado protegido no banco, com teste de duas requisições simultâneas (G2)
- [ ] Webhook com corpo bruto e ID único antes de agir; chamada externa com chave de idempotência; rota de escrita com `Idempotency-Key` (G3)
- [ ] Teste de integração na rota real: feliz, erro, repetição, limite e acesso indevido; nada pulado (G4)
- [ ] Tela sem regra, domínio sem I/O, arquivos dentro do limite (G5)
- [ ] Migration revisada; nada apagado; plano aprovado quando exigido (G6)
- [ ] Dois identificadores em toda tela ou documento clínico tocado; erro prevenido (R1); confirmação só em ato irreversível com o nome do paciente (R3)
- [ ] Mensagem de erro no formato R8 com `trace_id`; rascunho e reenvio sem perda (R6); atendimento segue sem dependência externa (R7)
- [ ] Domínios SEC-* tocados (B1 a B13) marcados e IDs citados (docs/17 Parte C)
- [ ] Metadados de lastro, eventos e ligações gravados e testados (docs/21 §5)
- [ ] Acessibilidade verificada; métricas, logs e traces instrumentados; orçamento p95 respeitado
- [ ] OpenAPI, ADR, manual e matriz de rastreabilidade atualizados; chave liga/desliga definida quando aplicável
- [ ] `just test` verde (ou falhas de bug real em `docs/bugs.md`)

## 8. Anti-padrões

- Calcular total, desconto ou permissão na tela e "confiar" no valor enviado.
- Esconder botão como se fosse segurança; rota sem permissão declarada.
- Checar se o horário está livre com `SELECT` e depois gravar sem constraint ou `UPDATE` condicional.
- `Mutex` ou lock em Redis para garantir exclusividade.
- Chamar PIX, operadora ou WhatsApp dentro da transação do banco.
- Processar webhook antes de verificar a assinatura ou de gravar o ID do evento; usar CPF como chave de idempotência.
- `UPDATE` ou `DELETE` em laudo assinado, evento ou registro clínico.
- `format!` montando SQL; `dangerouslySetInnerHTML`; token em `localStorage`.
- Mockar o banco no teste da regra; `#[ignore]` ou `skip` para passar; `sleep` em teste.
- Logar nome, CPF ou conteúdo clínico; colocar ID de paciente com dado sensível na URL.
- Corrigir bug "de passagem" durante refatoração; refatorar sem testes de caracterização.
- Introduzir cache, fila ou serviço novo sem medição e sem ADR.
- Colar dado de paciente, segredo ou código sensível em ferramenta externa (docs/14 B6).

## 9. Quem assina e quando escalar

| Situação | Quem |
|---|---|
| PR comum | Um revisor |
| PR em laudos, assinatura, faturamento, caixa, auditoria, identidade, exames e DICOM | Dois revisores (`CODEOWNERS`) |
| Migration | Revisor humano antes de rodar (G6) |
| Decisão estrutural (componente novo, tecnologia, padrão) | ADR assinado por tech lead e dono do produto |
| Mudança em fluxo clínico, alerta bloqueante novo | Responsável clínico de segurança |
| Frase de comportamento nova ou alterada | Dono do produto |
| Dados duplicados que impedem constraint | Reportar com proposta de limpeza; nunca apagar |
| Não deu para verificar algo | Escrever "não verificado" e o motivo |
| Escolha técnica com alternativas | Decidir pelo mais profissional e rastreável e registrar em `docs/decisoes.md` (ADR-0019) |

## 10. Leia a referência do assunto

| Quando você está | Leia |
|---|---|
| Escrevendo domínio, porta, adaptador ou rota em Rust | `references/backend-rust.md` |
| Criando tabela, migration, constraint, política RLS ou evento de lastro | `references/banco-migrations-e-rls.md` |
| Desenhando rota REST, erro, paginação, OpenAPI, `Idempotency-Key`, evento ou tempo real | `references/api-contratos-e-eventos.md` |
| Integrando Orthanc, aparelhos, TISS, NFS-e, PIX, WhatsApp, assinatura, Typst ou IA | `references/integracoes-dicom-e-documentos.md` |
| Escrevendo tela React | `references/frontend-react.md` |
| Escrevendo comando Tauri, periférico ou modo quiosque | `references/desktop-tauri.md` |
| Tratando de app móvel, portal responsivo ou tablet | `references/mobile.md` |
| Abrindo PR, escrevendo plano de correção, registrando bug ou refatorando | `references/qualidade-de-codigo-e-pr.md` |

Testes de tela seguem a skill `e2e` e o `docs/28`. Auditoria de segurança completa: `/security-audit`.
Roteiros de correção: `/auditoria`, `/testes-integracao`, `/refatoracao-segura`, `/seguranca`,
`/idempotencia`, `/condicao-de-corrida`.
