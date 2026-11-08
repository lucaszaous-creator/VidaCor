# 30 · Contrato da Fase 1: os 10 fluxos críticos (docs/27)

- **Status:** núcleo construído e testado (08/11/2026): os 10 fluxos, 138 testes de integração. Frases aprovadas pelo dono do produto (DEC-2026-11-08-49).
- **Diferenças da implementação em relação à primeira versão deste documento:** ver § 7.
- Complementa o contrato do M0 (`docs/29` § 6.2). O `openapi.json` gerado do núcleo é a fonte da
  verdade; este documento existe para núcleo e telas trabalharem em paralelo e é corrigido quando o
  gerado divergir.

## 0. Convenções (valem para todas as rotas)

- Erros: `application/problem+json` com `codigo` estável, `detail` no formato R8 e `trace_id`.
- Escrita: cabeçalho `X-CSRF-Token`; rotas que criam registro exigem `Idempotency-Key` (UUID).
- Estação: cabeçalho opcional `X-Estacao` (ex.: `GUICHE-1`, `SALA-1`), só letras, números e hífen,
  até 32 caracteres; gravado no lastro.
- **Reautenticação (SEC-AUT-5):** atos sensíveis respondem `401` com `codigo` `reautenticacao-necessaria`
  quando o último login com senha e 2FA tem mais de 5 minutos. A tela leva a
  `GET /v1/sessao/reautenticar?voltar=<caminho>` (o provedor pede senha e 2FA de novo) e repete o ato
  ao voltar, com a mesma `Idempotency-Key`. Atos: assinar laudo, retificar laudo, conduta da triagem,
  autorizar desconto, aprovar estorno, acesso de emergência, corrigir cadastro.
- Datas `AAAA-MM-DD`; instantes RFC 3339; dinheiro em **centavos** (inteiro); percentuais em pontos
  base (`500` = 5,00%).
- Situações e motivos sempre por palavra (não por cor).

## 1. Recepção, etiquetas e sala (FC-CHK, FC-ETQ, FC-PUL)

### Check-in (substitui o corpo do M0)
`POST /v1/atendimentos` (ato `recepcao.checkin`)
```json
{ "agendamento_id": "uuid",
  "confirmacao": { "nome": "texto", "nascimento": "AAAA-MM-DD", "documento_com_foto": true } }
```
- `201` `Atendimento` com `situacao: "aguardando_sala"`, `senha` (ex.: `"A012"`) e `pulseira`
  (`{ "codigo": "ETQ-2026-000001", "situacao": "valida" }`).
- `422` `identificacao-incompleta` (falta nome, nascimento ou documento com foto; a tela diz qual),
  `422` `identificacao-nao-confere`, `409` `checkin-ja-realizado`, `409` `situacao-nao-permite-checkin`,
  `403` `sem-permissao`.
- A worklist **não** é mais enviada no check-in: sai no bipe da pulseira na sala (FC-PUL-01).

### Agenda: alerta de homônimo
`GET /v1/agenda` ganha em cada item `homonimos: PacienteResumo[]` (outros pacientes do dia com o
mesmo nome e outro nascimento) e `requer_contraste: boolean`, `laudo_situacao: string | null`.

### Etiquetas
- `GET /v1/atendimentos/{id}` inclui `pulseira`, `senha`, `triagem` (ver § 2) e `laudo_situacao`.
- `POST /v1/etiquetas/{codigo}/reimpressao` (ato `etiqueta.reimprimir`, `Idempotency-Key`)
  `{ "motivo": "rasgou" }` responde `201` `{ "nova": Etiqueta, "invalidada": Etiqueta }`.
  `422` `motivo-obrigatorio`, `409` `etiqueta-ja-invalidada`, `403`.
- `Etiqueta`: `{ codigo, tipo: "pulseira", situacao: "valida" | "invalidada", motivo_invalidacao, substituida_por }`.

### Correção de cadastro (FC-ETQ-04)
`POST /v1/pacientes/{id}/correcao` (ato `paciente.corrigir`, reautenticação)
`{ "nascimento": "AAAA-MM-DD", "motivo": "texto" }` responde `200`
`{ paciente: PacienteResumo, versao, etiquetas_invalidadas: string[] }`. A tela pede a reimpressão.

### Sala: bipe da pulseira
`POST /v1/salas/{sala_id}/bipes` (ato `sala.bipar`, técnico lotado na sala; `Idempotency-Key`)
`{ "codigo_etiqueta": "ETQ-2026-000001" }`
- `200` `{ atendimento: Atendimento, aparelho: { nome, ae_title }, worklist: "enviada" | "ja_enviada" }`.
- `409` `etiqueta-substituida`, `409` `paciente-de-outra-sala` (com `sala_correta: { id, nome }` no
  corpo e divergência aberta), `422` `triagem-ausente` (exame com contraste sem triagem),
  `409` `conduta-medica-pendente`, `404` `etiqueta-nao-encontrada`, `403` (técnico de outra sala).
- `GET /v1/salas/{sala_id}/fila` (ato `sala.ver`): atendimentos do dia da sala com situação.

## 2. Triagem com contraste (FC-TRI)

`POST /v1/atendimentos/{id}/triagem` (ato `triagem.registrar`, técnico lotado na sala)
```json
{ "reacao_previa_contraste": false, "gestante": false, "amamentando": false,
  "usa_metformina": false, "creatinina_mg_dl": "1.4", "creatinina_data": "2026-10-28",
  "tfg": 90 }
```
- O servidor **ignora** `tfg` e calcula pela CKD-EPI 2021 com idade e sexo do cadastro (FC-TRI-07).
- `201` `Triagem`: `{ resultado: "liberado" | "liberado_com_ressalvas" | "precisa_aprovacao_medica",
  tfg_calculada, motivos: string[], regra_versao: "TRI-2026.1", conduta: null | Conduta }`.
- `POST /v1/atendimentos/{id}/triagem/conduta` (ato `triagem.conduta`: médico com vínculo de cuidado;
  reautenticação) `{ "conduta": "prosseguir com hidratação" }` responde `200` `Triagem` liberada.
  Técnico: `403`.

## 3. Agendamento de convênio com guia (FC-AGD)

- `POST /v1/guias` (ato `guia.registrar`, `Idempotency-Key`)
  `{ operadora, numero, carteirinha, senha, validade, paciente_id, solicitante, procedimentos: [{ tuss, quantidade }] }`
  responde `201` `Guia` `{ id, numero, operadora, validade, situacao: "disponivel" | "consumida", procedimentos: [{ tuss, saldo }], atendimento_codigo }`.
- `POST /v1/agendamentos` (ato `agenda.agendar`, `Idempotency-Key`)
  `{ paciente_id, sala_id, inicio, procedimentos: [tuss], guia_id | null, reserva_provisoria: boolean }`
  - `201` `ItemAgenda` (`situacao` `confirmado` ou `reserva_provisoria` com `expira_em`).
  - `409` `guia-em-uso` (com `atendimento_codigo`), `422` `procedimento-nao-autorizado` (com `tuss`),
    `422` `senha-vencida` (com `validade`), `422` `saldo-esgotado`, `409` `horario-ocupado` com
    `sugestoes: [inicio]` (próximos três horários livres da sala), `403`.
- `POST /v1/agendamentos/{id}/guia` `{ guia_id }`: confirma a reserva provisória (FC-AGD-09).
- Reserva provisória vence pelo trabalhador do núcleo: `situacao` `expirada`, motivo
  `guia não entregue no prazo`, horário volta a ficar livre, aviso ao paciente pelo outbox.
- `GET /v1/pacientes?busca=texto` (ato `agenda.ver`): até 20 pacientes, com nome e nascimento.
- `GET /v1/salas/{id}/horarios-livres?data=` : horários livres de 30 minutos.

## 4. Laudo, portal e acesso (FC-LAU, FC-ACE)

- `GET /v1/laudos/fila` (ato `laudo.escrever`): laudos atribuídos a mim; só estudo conciliado.
- `GET /v1/laudos/{id}` (ato `laudo.ler` + vínculo de cuidado ou acesso de emergência ativo):
  `Laudo` `{ id, codigo, paciente: PacienteResumo, exame, situacao: "rascunho" | "assinado" | "retificacao_em_rascunho",
  conversar_antes, publicado_no_portal, versao_vigente, versoes: [Versao], estudo }`;
  `Versao` `{ numero, texto, situacao: "rascunho" | "assinada" | "substituida", motivo, assinada_por, assinada_em, hash_documento }`.
  - Médico sem vínculo: `403` `sem-vinculo-de-cuidado` (a tela oferece o acesso de emergência).
  - Recepção e TI: `403` `sem-permissao`. A recepção vê só `laudo_situacao` no atendimento.
- `PUT /v1/laudos/{id}/rascunho` (só o médico atribuído) `{ texto, versao_numero }`: `409` `laudo-assinado`.
- `POST /v1/laudos/{id}/assinatura` (médico atribuído, reautenticação, `Idempotency-Key`)
  `{ "pin": "texto" }`: `200` `Laudo`; `403` sem atribuição; `409` `assinatura-concorrente`.
  A tela mostra antes: "Assinar o laudo de {nome} ({nascimento})?".
- `POST /v1/laudos/{id}/retificacao` `{ texto, motivo }`: `422` `motivo-obrigatorio`.
- `POST /v1/laudos/{id}/conversar-antes` `{ marcado: boolean }`; `POST /v1/laudos/{id}/liberacao` (depois da conversa).
- `POST /v1/laudos/{id}/acesso-emergencia` (reautenticação) `{ justificativa }` (mínimo 20 caracteres): vale 1 hora.
- Publicação no portal: `409` `laudo-nao-assinado` para rascunho (FC-LAU-02).

### Portal do paciente (`/v1/portal/*`, sessão de paciente separada da equipe)
- `GET /v1/portal/laudos`: laudos publicados do próprio paciente (versão vigente, marca de retificação).
- `GET /v1/portal/laudos/{id}`: `404` `nao-encontrado` igual para "de outro paciente" e "não existe" (FC-ACE-02).
- `GET /v1/links/{token}`: laudo compartilhado; `410` `link-expirado`.

## 5. Caixa e PIX (FC-CXA, FC-PIX)

- `POST /v1/caixas` (ato `caixa.operar`) `{ fundo_centavos }`; `POST /v1/caixas/{id}/fechamento` `{ contado_centavos }`
  responde `{ esperado_centavos, contado_centavos, diferenca_centavos }`.
- `GET /v1/caixas/meu`: caixa aberto do usuário ou `null`.
- `POST /v1/recebimentos` (ato `caixa.receber`, `Idempotency-Key`)
  `{ paciente_id, atendimento_id, itens: [{ tuss, valor_centavos? }], desconto: { pontos_base, motivo, autorizacao_id? } | null, forma: "pix" | "dinheiro" | "cartao" }`
  - O servidor calcula o total pela tabela vigente e grava a versão (FC-CXA-01, 02).
  - `201` `Recebimento` `{ id, codigo, itens: [{ tuss, descricao, valor_centavos }], subtotal_centavos, desconto_centavos, total_centavos, tabela_versao, situacao: "aguardando_pagamento" | "pago" | "estornado", pix: { txid, copia_e_cola } | null }`.
  - `403` `desconto-acima-do-limite` (com `limite_pontos_base`), `422` `valor-invalido`, `409` `caixa-fechado`.
- `POST /v1/autorizacoes-desconto` (ato `desconto.autorizar`, reautenticação) `{ paciente_id, pontos_base, motivo, solicitante_id }`: `201` `{ id, valida_ate }`.
- `POST /v1/recebimentos/{id}/estornos` `{ motivo }` (pedido); `POST /v1/estornos/{id}/aprovacao` (ato `estorno.aprovar`, reautenticação; quem pediu não aprova: `403` `separacao-de-funcoes`).
- `POST /v1/webhooks/pix` (público; assinatura HMAC-SHA256 do corpo bruto no cabeçalho `X-Assinatura`):
  `401` `assinatura-invalida`; `200` sempre que o evento é aceito ou já foi processado.

## 6. Contingência (FC-CTG)

- `POST /v1/contingencias` (ato `contingencia.declarar`) `{ nivel: "externo" | "setor" | "total", motivo, responsavel_id, declarada_em, chave }`
  (`chave` UUID gerada na estação; repetir não cria outra): `201` `Contingencia`.
- `GET /v1/contingencias/aberta`.
- `POST /v1/contingencias/kits` (ato `contingencia.kit`): reserva a faixa do dia e gera o kit com hash.
- `POST /v1/contingencias/{id}/fichas` `{ formulario: "F-ATD", numero, paciente_id, nome, nascimento, ocorrido_em, atendido_por, anexo_sha256 }`:
  `201` atendimento com `origem: "contingencia"`; `409` `ficha-ja-lancada`; `422` `numero-fora-da-faixa`;
  `202` `divergencia-aberta` quando o nascimento do papel difere do cadastro.
- `POST /v1/contingencias/{id}/recibos` `{ formulario: "F-REC", numero, paciente_id, tuss, valor_papel_centavos, ocorrido_em }`:
  grava o valor recalculado e abre divergência de preço quando difere.
- `POST /v1/contingencias/{id}/encerramento`: `409` `pendencias` com a lista do que falta.

## 7. Ajustes feitos na implementação (valem sobre o texto acima)

| Onde | Ajuste |
|---|---|
| Recepção | `confirmacao.nome`, `nascimento` e `documento_com_foto` são opcionais no corpo para o servidor dizer qual falta (`identificacao-incompleta`, com o item no `detail`). |
| Sala | `POST /v1/salas/{sala_id}/bipes` exige `Idempotency-Key`; `409 paciente-de-outra-sala` traz `sala_correta` e `divergencia`. Técnico fora da sala: `403 fora-da-sala`. Sala sem aparelho: `409 sala-sem-aparelho`. |
| Triagem | `triagem-ausente` é 422 no bipe; conduta de médico sem vínculo: `403 sem-vinculo-de-cuidado`. Triagem também exige o sexo no cadastro (`422 cadastro-sem-sexo`). |
| Agendamento | `situacao` do item da agenda inclui `reserva_provisoria` e `expirada` (expirada não aparece na agenda); o item traz `expira_em`. Procedimentos vêm do catálogo (`procedimento-desconhecido`). |
| Laudo | `POST /v1/laudos/{id}/publicacao` (manual, 409 para rascunho); `POST /v1/laudos/{id}/links` `{ horas }` devolve o token uma única vez. Sem serviço de assinatura configurado: `503 assinatura-indisponivel`. PIN recusado: `422 pin-recusado`. Laudo de outro médico: `403 laudo-de-outro-medico`. |
| Estudos | `GET /v1/estudos/pendentes` e `POST /v1/estudos/{id}/conciliacao` `{ atendimento_id, nome, nascimento }` (médico; conciliação manual com dois identificadores). |
| Caixa | O `pix` do recebimento vem `null` até o PSP devolver a cobrança pelo outbox: a tela consulta `GET /v1/recebimentos/{id}` até aparecer o copia-e-cola. Sem tabela de preços vigente: `409 sem-tabela-de-precos`. |
| Contingência | A ficha F-ATD leva `agendamento_id` (a agenda impressa no kit traz o código). Novas rotas: `POST /v1/contingencias/{id}/numeros` `{ formulario, de, ate, situacao: "devolvido" \| "anulado" }` e `POST /v1/divergencias/{id}/resolucao` `{ resolucao }` (gerência). A lista do `409 pendencias` vem em `pendencias: string[]`. |
| Sala | `GET /v1/salas` devolve `SalaDaUnidade` com `lotado` (o usuário da sessão está lotado na sala). A tela do técnico abre na primeira sala lotada; o servidor continua recusando bipe e fila fora da lotação (`403`). Achado na verificação contra o núcleo real. |
| Pendências do caixa | Fila em vez de link no chat (DEC-2026-11-08-55): `POST /v1/pedidos-desconto` (caixa, acima do limite, `Idempotency-Key`; `409 pedido-ja-aberto`), `GET /v1/pedidos-desconto[?situacao=]` (gerência: unidade; caixa: só os próprios, RLS), `GET /v1/pedidos-desconto/{id}`, `POST …/{id}/recusa` `{ motivo }` (gerência) e `…/{id}/cancelamento` (só quem pediu). `POST /v1/autorizacoes-desconto` aceita `pedido_id` e fecha o pedido na mesma transação (`409 pedido-ja-decidido`). Estornos: `GET /v1/estornos[?situacao=]`, `GET /v1/estornos/{id}`, `POST /v1/estornos/{id}/recusa` `{ motivo }`; recusado pode ser pedido de novo. Tela `/caixa/pendencias`; os endereços antigos redirecionam. |
| Menu de contexto | `GET /v1/menu/agendamento/{id}`, `/v1/menu/atendimento/{id}` (fila da sala) e `/v1/menu/laudo/{id}` devolvem `{ cabecalho, acoes[] }` com `id`, `rotulo`, `estado`, `motivo`, `perigo` e `destino` (tela que faz a ação, só com UUID). Ação que o perfil nunca terá não vem; indisponível vem com motivo; destrutivas no fim (DEC-2026-11-08-56). Objeto de outra unidade: `404`. |
| Portal: convite e login | `POST /v1/pacientes/{id}/convites-portal` (recepção, gerência; `Idempotency-Key`; o link vai por e-mail ao endereço do cadastro pelo outbox `email.convite_portal` e a resposta traz só `enviado_para` mascarado; sem e-mail: `422 paciente-sem-email`; servidor sem segredo: `503`, DEC-2026-11-08-60), `GET /v1/portal/convites/{token}` (`valido` ou `indisponivel`, sem dado pessoal), `POST /v1/portal/convites/{token}/aceite` `{ nascimento, login, senha }` (uso único, 5 erros revogam, `410 convite-indisponivel`), `POST /v1/portal/sessao` `{ login, senha }` (`401 login-nao-confere` único; `429 acesso-em-espera` após 5 erros), `GET /v1/portal/sessao`, `POST /v1/portal/sessao/encerramento` (DEC-2026-11-08-57). |
| Cadastro de paciente | `GET /v1/pacientes/possiveis-duplicados?nome&nascimento[&cpf]` e `POST /v1/pacientes` (recepção, gerência; `Idempotency-Key`) com nome, nome social, nascimento, sexo, CPF (opcional, conferido e único), nome da mãe, celular, e-mail, alergias ou `nega_alergias`. Parecido não conferido: `409 possivel-duplicado` com `candidatos`; mesmo CPF: `409 cpf-ja-cadastrado` sempre. Regra no domínio (`dominio_recepcao::cadastro`): mesmo CPF; mesmo nome e nascimento; mesmo nome com dia e mês trocados, um dígito diferente ou dígitos vizinhos invertidos; mesmo nascimento com primeiro e último nome iguais (DEC-2026-11-08-58). |
| Caixa: a receber | `GET /v1/caixa/a-receber?data=` (só caixa): atendimentos do dia com paciente, horário, procedimentos pela tabela de preços (`tuss`, `descricao`, `valor_centavos`) e `recebido`. O caixa não lê `/v1/agenda` (N3) (DEC-2026-11-08-59). |
| Estoque | `GET /v1/estoque/itens` (lotes na ordem de saída, saldo dentro da validade), `POST /v1/estoque/entradas` (estoque, gerência; lote soma; validade diferente: `409 validade-diferente`; vencido: `422`), `POST /v1/atendimentos/{id}/contraste` `{ item_id, quantidade }` (técnico/enfermagem lotados; triagem liberada; o servidor escolhe o lote que vence antes; `409 sem-lote-valido`, `sem-triagem`, `triagem-pede-conduta`), `GET /v1/estoque/lotes/{id}/rastreio` (gerência; leitura no lastro) (DEC-2026-11-08-61). |
| Lastro | `GET /v1/lastro/cadeia?codigo=` (gerência, auditor): qualquer código do atendimento (ATD, accession, AGD, ETQ, guia, TRI, LAU, EST/StudyUID, REC, estorno) devolve paciente, registros ligados e eventos com `hash` e `integro` (recalculado no servidor); sem `dados` do evento. Código desconhecido ou de outra unidade: `404` igual. A leitura vira evento `cadeia_lida` (DEC-2026-11-08-62). |
| Laudo estruturado (E2) | `GET /v1/laudos/{id}/achados` (médico com vínculo; leitura auditada) devolve `AchadosDoLaudo` da versão em rascunho ou da vigente, com `editavel`, `texto_achados`, `texto_conclusao` e `regras_vigentes`. `POST /v1/laudos/{id}/achados` (médico atribuído, `Idempotency-Key`) com `sistema` (`tirads` ou `birads`), `localizacao`, `maior_diametro_mm` e os descritores do sistema: o servidor devolve `201 Achado` com `pontos`, `categoria`, `categoria_nome`, `conduta`, `regra_versao` e textos sugeridos; categoria enviada pela tela é ignorada. `POST /v1/laudos/{id}/achados/{achado_id}/remocao` marca como removido (repetir devolve a mesma lista). Laudo assinado: `409 laudo-assinado`; coerência: `422` com o código do domínio (`birads-6-sem-biopsia`, `categoria-incoerente`, `medida-invalida`...); `422 limite-de-achados` a partir de 20. A assinatura inclui os achados no hash; a retificação leva os achados para a nova versão (DEC-2026-11-08-64). |
| Indicadores | `GET /v1/indicadores?de&ate` (gerência, financeiro; até 366 dias): agendamentos, atendimentos, faltas e taxa, receita do particular e ticket médio, tempo médio de laudo (estudo recebido até a 1ª assinatura) por médico, produção por exame e ocupação por dia da semana e hora. Só agregados, sem paciente (DEC-2026-11-08-66). |
| WhatsApp | Avisos pelo outbox (`aviso.confirmacao`, `aviso.laudo_pronto`, `aviso.reserva_expirada`) pela porta `Mensageiro`, só com primeiro nome, data e hora (sem exame); cada mensagem registrada em `comunicacao.mensagem` (enviada, bloqueada por opt-out ou sem celular). `POST /v1/comunicacao/lembretes {data}`, `GET/POST /v1/pacientes/{id}/comunicacao` (recepção, gerência; troca com motivo). `POST /v1/webhooks/whatsapp` com `X-Hub-Signature-256`: "SIM"/"1" confirma o próximo horário, "SAIR" desliga (DEC-2026-11-08-67). |
| Financeiro | `GET/POST /v1/financeiro/titulos` (lançar com `Idempotency-Key`; a pagar exige 1 aprovação, 2 acima de R$ 10 mil), `POST …/titulos/{id}/aprovacoes` (reautenticação; quem lançou não aprova), `POST …/titulos/{id}/baixa` `{ conta_id, data, extrato_linha_id? }` (única; extrato confere valor, sinal e conta), `POST /v1/financeiro/extratos` (ID do banco não duplica), `GET /v1/financeiro/resumo`, `GET /v1/financeiro/contas`, repasse: `GET /v1/financeiro/repasse?competencia=AAAA-MM`, `POST …/repasse/{competencia}/titulos` (um por médico e competência; competência aberta: `422`), `POST /v1/financeiro/regras-repasse` (nova versão, reautenticação) (DEC-2026-11-08-68). |
| Usuários e perfis | `GET/POST /v1/usuarios` (TI e gerência; criar com reautenticação e `Idempotency-Key`; com perfil clínico nasce inativo e aguarda), `POST /v1/usuarios/{id}/mudancas` `{ tipo: conceder\|retirar\|desativar\|reativar, perfil?, motivo }` (reautenticação; ninguém muda a própria conta; conceder clínico e reativar aguardam; desativar derruba a sessão), `POST /v1/usuarios/pedidos/{id}/decisao` `{ aprovar, motivo }` (gerência, segunda pessoa). Separação de funções: TI sem perfil clínico (também por gatilho no banco); caixa sem financeiro (DEC-2026-11-08-69). |
| NFS-e | Recebimento pago (dinheiro e cartão na hora; PIX na confirmação) enfileira `nfse.emitir`; estorno aprovado enfileira `nfse.cancelar`. Porta `Prefeitura` (simulada nos testes), uma nota por recebimento, descrição genérica sem o exame. `GET /v1/recebimentos/{id}/nota` (`404 nota-nao-emitida` até sair) (DEC-2026-11-08-70). |
| Exame externo (E3) | `GET/POST /v1/pacientes/{id}/exames-externos` (recepção e gerência; `Idempotency-Key`): nome DICOM compatível e nascimento igual ao cadastro, documento conferido; mesmo StudyUID não entra duas vezes no paciente; registro imutável, sempre só leitura, nunca vira laudo da VidaCor; evento de lastro `exame_externo_importado` (DEC-2026-11-08-72). |
| TV de chamada | `POST /v1/atendimentos/{id}/chamadas` `{ destino: guiche\|sala, guiche? }` (recepção ao guichê 1 a 20; técnico lotado à sala; `Idempotency-Key`; rechamar é outra chamada), `GET /v1/painel-de-chamada` (perfil `tv`, recepção e gerência; `{ agora, ultimas[4] }` só com senha, nome abreviado, destino e hora; `no-store`). A TV não lê agenda, atendimento, equipe nem eventos (DEC-2026-11-08-73). |
| Faturamento TISS (E5) | Atos novos: `FaturamentoVer` (faturista, gerente), `FaturamentoAuditar`, `LoteGerar`, `DemonstrativoRegistrar` e `GlosaRecorrer` (faturista). `POST /v1/faturamento/conferencias` (`Idempotency-Key`) apura as contas novas e confere todas as abertas: `ResultadoConferencia`. `GET /v1/faturamento/contas` (`?situacao=`, `?lote_id=`), `GET /v1/faturamento/contas/{id}` (`ContaDetalhe` com pendências, versões e glosas). `POST /v1/faturamento/contas/{id}/correcao` (`Idempotency-Key`, `PedidoCorrecaoConta` com motivo): `409 conta-em-lote`; `422 motivo-invalido`, `solicitante-invalido`, `cid-invalido`, `correcao-vazia`. `GET /v1/faturamento/auditoria` (`PainelAuditoria`), `GET /v1/faturamento/contratos`. `POST /v1/faturamento/lotes` (`Idempotency-Key`, `{contrato_id}`): `201 LoteDetalhe`; `409 nenhuma-guia-pronta`; `404 contrato-nao-encontrado`. `GET /v1/faturamento/lotes`, `GET /v1/faturamento/lotes/{id}` (com `envio_pendente` quando o outbox ainda não entregou), `GET /v1/faturamento/lotes/{id}/arquivo` (XML ISO-8859-1). `POST /v1/faturamento/lotes/{id}/envio` (`202`, repetir não cria outro envio). `POST /v1/faturamento/lotes/{id}/demonstrativo` (`Idempotency-Key`): `409 lote-nao-enviado`, `demonstrativo-ja-registrado`; `422 valores-nao-fecham`, `glosa-invalida`, `demonstrativo-incompleto`, `data-futura`. `GET /v1/faturamento/glosas`, `POST /v1/faturamento/glosas/{id}/recurso` (`Idempotency-Key`): `409 recurso-ja-registrado`; `422 justificativa-invalida`, `prazo-de-recurso-vencido` (DEC-2026-11-08-65). |
| Geral | Erros podem trazer campos extras ao lado do Problem Details (`sugestoes`, `tuss`, `validade`, `atendimento_codigo`, `limite_pontos_base`). O `openapi.json` gerado está em `packages/api-client/openapi.json` (54 rotas). |
