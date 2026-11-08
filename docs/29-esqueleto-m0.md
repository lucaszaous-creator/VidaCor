# 29 · Documento de design do esqueleto (marco M0)

- **Autor:** Claude (papéis desenvolvimento e devops-seguranca), a pedido do dono do produto
- **Data:** 08/11/2026 · **Status:** em construção
- **Aprovações:** dono do produto [x] início da construção (DEC-2026-11-08-43) · responsável clínico de segurança [ ] (check-in é ato clínico: frases FC-CHK do docs/27 aguardam aprovação) · dono técnico [ ]

## 1. Comunicado (de trás para frente)
A recepção abre o VidaCor, entra com o seu usuário, vê a agenda do dia por sala e faz o check-in do paciente
confirmando nome completo e data de nascimento. O paciente entra na lista de trabalho do aparelho da sala, e
quando o aparelho envia as imagens, a tela mostra que o estudo chegou, sem recarregar. É a primeira fatia de
ponta a ponta (docs/11 § 11 item 5) e nasce com as regras do projeto: servidor decide, banco protege, repetir não
duplica, lastro em todo registro, RLS e testes de integração de verdade.

## 2. Perguntas frequentes
- **Quem usa?** Recepção (check-in) e qualquer perfil autorizado (agenda). Perfis do docs/18.
- **Como sabe que deu certo?** O agendamento passa a "em atendimento", aparece o código do atendimento e o accession, e a sala recebe o paciente na worklist.
- **O que fica fora?** Agendamento com guia, caixa, laudo, portal (Fase 1 em diante).

## 3. Requisitos
| ID | Requisito | Critério de aceite |
|---|---|---|
| REQ-IDT-001 | Toda rota exige sessão válida, exceto saúde e entrada | Sem sessão: 401 em Problem Details |
| REQ-IDT-002 | Toda rota declara o ato exigido; ato não concedido ao perfil responde 403 | Teste de acesso indevido por rota |
| REQ-IDT-003 | Sessão em cookie HttpOnly, Secure, SameSite=Strict; escrita exige cabeçalho CSRF | Escrita sem CSRF: 403 |
| REQ-AGD-001 | Agenda do dia por sala, ordenada por horário, com dois identificadores do paciente | Rota devolve os agendamentos do dia e da unidade do usuário |
| REQ-REC-001 | Check-in só com nome completo e nascimento iguais aos do cadastro (dois identificadores) | Divergência: 422, nada é gravado |
| REQ-REC-002 | Um agendamento tem no máximo um check-in, mesmo com duas requisições simultâneas | Índice único; segunda requisição: 409 |
| REQ-REC-003 | Repetir o check-in com a mesma `Idempotency-Key` devolve a mesma resposta | Mesma resposta, um só atendimento |
| REQ-REC-004 | Check-in gera atendimento (ATD-AAAA-NNNNNN) e accession (ACC-AAAA-NNNNNN) únicos e sequenciais no ano | Sequência no banco, sem buraco por corrida |
| REQ-REC-005 | Check-in grava evento imutável com hash encadeado e item no outbox para a worklist | Evento e outbox gravados na mesma transação |
| REQ-EXA-001 | O item de worklist chega ao Orthanc pelo outbox, com retentativa e idempotência | Worklist consultável no Orthanc |
| REQ-EXA-002 | Estudo recebido é conciliado pelo accession; sem conciliação, não vai para laudo | Estudo com accession desconhecido fica pendente de conciliação |
| REQ-EXA-003 | A tela recebe o aviso de estudo recebido por SSE, só para quem pode ver | Evento autorizado por usuário |
| REQ-LAS-001 | Toda tabela tem os metadados de lastro do docs/21 § 1 | Teste de schema |
| REQ-LAS-002 | Tabelas com dado de paciente têm RLS forçado; contexto por `SET LOCAL` | Teste autorizado e não autorizado |

## 4. Telas
`design/mockups/Agenda.dc.html` (agenda do dia), `design/mockups/Recepcao.dc.html` e `Fluxo-Completo.dc.html`
etapa 2 (check-in com dois identificadores), `Login.dc.html`, `Sidebar.dc.html`, `Barra-Paciente.dc.html`.
Tarefas críticas: T-REC-03 e T-REC-04 (docs/26).

## 5. Riscos
RC-001 (paciente errado: dois identificadores obrigatórios, alerta de homônimo na tela), RC-002 (imagem no
paciente errado: accession gerado pelo servidor, conciliação), RC-008 (exposição: RLS, permissão por ato,
nada de dado de saúde em log). STRIDE resumido: falsificação (sessão HttpOnly, CSRF), adulteração (servidor
recalcula, evento com hash), repúdio (lastro), vazamento (RLS, `no-store`), negação de serviço (limites de corpo
e tempo), elevação (ato por rota, negar por padrão).

## 6. Design técnico

### 6.1 Componentes
| Caminho | O que é |
|---|---|
| `crates/comum` | Erros (Problem Details RFC 9457), ids (UUIDv7 e código legível), tempo, tipos de valor (`NomeCompleto`, `DataNascimento`) |
| `crates/dominio-recepcao` | Regra do check-in, sem I/O (G5) |
| `crates/api` | Binário axum: rotas, sessão, permissão, adaptadores PostgreSQL e Orthanc, outbox, SSE, OpenAPI (utoipa) |
| `infra/compose/dev.yml` | PostgreSQL 17, Orthanc com worklist, NATS, Keycloak |
| `packages/design-system` | Tokens e componentes do `design/vc10.css` (ADR-0021) |
| `packages/api-client` | `openapi.json` gerado do código e cliente TypeScript gerado |
| `apps/web` | Telas React |

### 6.2 Contrato HTTP (v1)
Erros sempre `application/problem+json` com `type`, `title`, `status`, `detail` (pt-BR, formato R8), `codigo` estável
e `trace_id`. Respostas com dado de paciente levam `Cache-Control: no-store`.

| Método e rota | Ato | Entrada | Saída |
|---|---|---|---|
| `GET /v1/saude` | público | | `{ "situacao": "ok" }` |
| `GET /v1/sessao` | sessão | | `{ usuario: { id, nome, perfis[] }, unidade: { id, nome }, csrf }` |
| `GET /v1/sessao/entrar` | público | | redireciona ao Keycloak (OIDC com PKCE, state e nonce) |
| `GET /v1/sessao/retorno` | público | `code`, `state` | cria a sessão e redireciona à aplicação |
| `POST /v1/sessao/sair` | sessão + CSRF | | 204 |
| `GET /v1/salas` | `agenda.ver` | | `[{ id, nome }]` |
| `GET /v1/agenda?data=AAAA-MM-DD&sala_id=` | `agenda.ver` | | `{ data, itens: [{ id, codigo, inicio, fim, sala: {id, nome}, paciente: {id, nome, nascimento}, exame: {codigo_tuss, nome}, situacao, atendimento_id }] }` |
| `POST /v1/atendimentos` | `recepcao.checkin` + CSRF + `Idempotency-Key` | `{ agendamento_id, confirmacao: { nome, nascimento } }` | 201 `{ id, codigo, accession, agendamento_id, paciente: {id, nome, nascimento}, situacao, criado_em }` |
| `GET /v1/atendimentos/{id}` | `recepcao.ver` (por objeto) | | atendimento com `estudo` (`null` ou `{ study_uid, imagens, recebido_em }`) |
| `GET /v1/eventos` | sessão | | SSE; evento `estudo-recebido` `{ atendimento_id, accession, imagens }` |

Situações do agendamento: `agendado`, `confirmado`, `em_atendimento`, `concluido`, `faltou`, `cancelado`.
Códigos de erro do check-in: `identificacao-nao-confere` (422), `checkin-ja-realizado` (409),
`agendamento-nao-encontrado` (404), `situacao-nao-permite-checkin` (409), `idempotency-key-ausente` (400).

### 6.3 Dados
Schemas por módulo (`identidade`, `agenda`, `recepcao`, `exames`, `lastro`, `integracao`). Toda tabela com
`id` UUIDv7, `codigo`, `criado_em`, `criado_por`, `origem`, `estacao`, `unidade_id`, `versao`, `correlacao`.
Índice único `recepcao.atendimento(agendamento_id)` (G2). Tabela `integracao.idempotencia(chave, rota, usuario_id)`
com resposta gravada (G3). `lastro.evento` append-only com `hash` e `hash_anterior`. `integracao.outbox`.
RLS forçado em `agenda.agendamento`, `recepcao.atendimento`, `identidade.paciente`, com `SET LOCAL app.usuario_id`,
`app.unidade_id`, `app.perfis`.

### 6.4 Garantias aplicadas
G1 (servidor confere identificação e gera códigos), G2 (índice único e sequência), G3 (`Idempotency-Key`; outbox
com chave por item), G4 (testes na rota real com PostgreSQL real), G5 (domínio sem I/O), G6 (migrations revisadas
no PR).

## 7. Lançamento
Nenhum: o M0 roda em `dev` e homologação com dados fictícios (`design/dados-ficticios.md`).

## 8. Fora do escopo
Guia de convênio, autorização, caixa, etiquetas, laudo, portal, desktop Tauri, app móvel.
