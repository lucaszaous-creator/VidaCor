# Concorrência, idempotência e acesso indevido

Fonte: `CLAUDE.md` G2 e G3, `docs/15` G2 e G3, `docs/17` B2, B3, B4, B8, `docs/18`, `docs/27`
(FC-AGD, FC-CXA, FC-PIX, FC-ETQ, FC-ACE).

## 1. G2: duas requisições simultâneas

Recursos limitados do VidaCor (todos precisam de teste): horário de sala, senha da fila, caixa
aberto, recebimento, lote de estoque, número de guia e de lote TISS, consumo de guia, convite do
portal, conciliação de estudo, versão de laudo, cortesia, faixa de numeração da contingência,
etiqueta.

Receita:
1. Preparar o estado com o recurso disponível uma única vez (saldo 1, horário livre).
2. Disparar as duas requisições **ao mesmo tempo**, com chaves de idempotência diferentes (são
   operações de negócio diferentes).
3. Conferir: exatamente uma resposta de sucesso, a outra 409 (ou 422 conforme a frase); no banco,
   um efeito só e nenhum saldo negativo.
4. Repetir o teste algumas vezes no mesmo processo (laço de 20) para aumentar a chance de
   intercalação; uma falha em 20 é bug.

```rust
// pseudocódigo
#[tokio::test]
async fn fc_agd_03_dois_agendamentos_simultaneos_com_a_mesma_guia() {
    let app = AppDeTeste::subir().await;
    let guia = app.guia("UN-55120", saldo(&[("40901114", 1), ("40901300", 1)])).await;
    let ana = app.usuario("Ana Souza", Perfil::Recepcao).await;
    let outra = app.usuario("Bruna", Perfil::Recepcao).await;

    let (a, b) = tokio::join!(
        app.agendar(&ana,   guia.id, "sala-1", "2026-11-07T09:00").idempotency_key(Uuid::now_v7()).enviar(),
        app.agendar(&outra, guia.id, "sala-2", "2026-11-07T09:30").idempotency_key(Uuid::now_v7()).enviar(),
    );

    let status = [a.status(), b.status()];
    assert!(status.contains(&201) && status.contains(&409));
    assert_eq!(app.db.agendamentos_com_guia(guia.id).await.len(), 1);
    assert!(app.db.saldo_guia(guia.id).await.iter().all(|s| *s >= 0));
}
```

Se o teste passar com a proteção removida (comente o índice único numa branch local), ele não está
exercitando a corrida: use uma barreira (`tokio::sync::Barrier`) antes do envio ou um ponto de
espera injetado entre a leitura e a gravação.

## 2. G3: `Idempotency-Key` nas rotas de escrita

Rotas que exigem: criar cobrança, recebimento, guia, agendamento, laudo, etiqueta, lançamento de
contingência. Casos obrigatórios:

| Caso | Esperado |
|---|---|
| Mesma chave, mesmo corpo, em sequência | Mesma resposta (status e corpo); um registro |
| Mesma chave, mesmo corpo, ao mesmo tempo | Um registro; as duas respostas iguais ou a segunda 409 "em processamento", conforme o contrato |
| Mesma chave, corpo diferente | 422 (chave reutilizada com outro pedido) |
| Sem chave em rota que exige | 400 |
| Chave com dado pessoal | Proibido no cliente; teste de revisão confere que a chave é UUID |

```rust
// pseudocódigo: FC-CXA-06
let chave = Uuid::now_v7();
let r1 = app.receber(&carla, &pedido).idempotency_key(chave).enviar().await;
let r2 = app.receber(&carla, &pedido).idempotency_key(chave).enviar().await;
assert_eq!(r1.corpo(), r2.corpo());
assert_eq!(app.db.recebimentos_do(sonia.id).await.len(), 1);
assert_eq!(app.fake_psp.cobrancas_criadas().await.len(), 1);   // chamada externa também uma vez
```

Chamadas que fazemos a provedores: o fake confere que a mesma chave de idempotência UUID foi
reenviada nas retentativas (derrube o fake na primeira tentativa e confira a segunda).

## 3. G3: webhooks recebidos

Para PIX, WhatsApp, NFS-e e provedor de assinatura:

| Caso | Esperado | Frase |
|---|---|---|
| Assinatura válida | ID do evento gravado antes de agir; 2xx; efeito uma vez | FC-PIX-01 |
| Duplicado | 200 e para; um efeito; um pedido de NFS-e | FC-PIX-02 |
| Assinatura que não confere com o corpo bruto | 401; nenhum evento gravado; nada muda | FC-PIX-03 |
| Corpo reformatado (mesmo JSON, espaços diferentes) com assinatura do original | 401 (prova que verifica o corpo bruto) | FC-PIX-03 |
| Fora de ordem | Consulta o estado atual no provedor; não regride | FC-PIX-04 |
| Dois iguais ao mesmo tempo | Um processado | FC-PIX-05 |
| Processamento lento | Responde 2xx rápido; trabalho segue pelo outbox | G3 |

```rust
// pseudocódigo: FC-PIX-03 webhook forjado
let corpo = br#"{"evento_id":"evt-1","txid":"...","status":"CONCLUIDA"}"#;
let resp = app.post("/v1/webhooks/pix")
    .header("x-assinatura", assinar_com_chave_errada(corpo))
    .body_bruto(corpo).enviar().await;
assert_eq!(resp.status(), 401);
assert!(app.db.eventos_recebidos("evt-1").await.is_empty());
assert_eq!(app.db.recebimento("REC-2026-004410").await.situacao, "aguardando");
```

Também conferir: comparação da assinatura em tempo constante é item de revisão de código
(SEC-CRI-2), não de teste de tempo.

## 4. Acesso indevido (SEC-ACS-4)

Toda rota com ID tem pelo menos:

| Caso | Esperado |
|---|---|
| Usuário A pede objeto de B pelo ID | 403 ou 404 conforme a frase; nada muda; tentativa auditada |
| A tenta alterar objeto de B | Mesmo resultado; banco sem alteração |
| Perfil sem o ato (Financeiro reimprimindo pulseira) | 403 (FC-ETQ-05) |
| Recepção pede conteúdo de laudo | 403; vê só a situação (FC-ACE-06) |
| TI pede laudo, evolução ou imagem | 403 (FC-ACE-07) |
| Médico sem vínculo | 403 e oferta de acesso de emergência (FC-ACE-03) |
| Acesso de emergência com justificativa | Lê; registro em destaque; aviso ao responsável clínico; fila de 48 h (FC-ACE-04) |
| Link compartilhado expirado | Nada exibido (FC-ACE-08) |
| Usuário de outra unidade | Não vê (`docs/18` § 6 item 4) |

```rust
// pseudocódigo: FC-ACE-01 e FC-ACE-02 no portal
let a = app.paciente_portal("Sônia Maria Castro", "1962-02-03").await;
let laudo_b = app.laudo_liberado_de("Antônio Carlos Ribeiro", "1954-06-15").await;

let existe     = app.get(&format!("/v1/portal/laudos/{}", laudo_b.id)).como(&a).enviar().await;
let inexistente = app.get(&format!("/v1/portal/laudos/{}", Uuid::now_v7())).como(&a).enviar().await;

assert_eq!(existe.status(), 404);
assert_eq!(existe.status(), inexistente.status());
assert_eq!(existe.corpo_sem_trace_id(), inexistente.corpo_sem_trace_id()); // sem oráculo (SEC-DAD-4)
assert!(app.db.auditoria_tentativa(a.id, laudo_b.id).await.is_some());
```

## 5. Autenticação e sessão

- **Step-up (SEC-AUT-5):** assinar laudo, estornar, autorizar desconto, exportar e mudar permissão
  sem reautenticação recente respondem 401 com pedido de segundo fator; com reautenticação, passam
  (FC-CXA-05). Não existe caminho que pule o 2FA.
- **Separação de funções:** quem pede estorno não aprova (FC-CXA-09); perfil clínico novo exige
  segunda pessoa.
- **CSRF (SEC-AUT-4):** escrita com cookie e sem token CSRF válido é recusada; com token de outra
  sessão, recusada.
- **Sessão (SEC-AUT-3):** após logout, troca de senha ou remoção de permissão, o token antigo é
  recusado na rota e no WebSocket.
- **Limite antes do login (SEC-DIS-5):** N tentativas erradas bloqueiam pelo prazo configurado;
  mensagem igual para usuário existente e inexistente (SEC-AUT-6).
- **`no-store` (SEC-AUT-8):** respostas com dado pessoal ou de saúde têm `Cache-Control: no-store`.
- **WebSocket (SEC-CLI-5):** usuário recebe só eventos que pode ver; conectar sem autenticação ou de
  origem não permitida falha.

## 6. Quando a proteção não existe ainda

O teste é escrito e **falha**. O bug vai para `docs/bugs.md` com o teste que reproduz. A correção
segue a ordem de preferência do G2 (índice único, operação atômica, `SELECT ... FOR UPDATE`) e é
outro PR. Proibido "proteger" com lock em memória ou Redis para o teste passar.
