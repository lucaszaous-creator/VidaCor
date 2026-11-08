# Testes de integração G4

Fonte: `docs/15-padrao-de-qualidade.md` G4, `docs/14` § B4, `docs/27`, `docs/18` § 7,
`docs/21` § 5, `docs/23` § 13, playbook `docs/playbooks/testes-integracao.md`.

## 1. Ambiente

- Rust: `cargo nextest` + Testcontainers com PostgreSQL 17, Orthanc e NATS JetStream quando o fluxo
  usa. Servidor axum subido em porta aleatória; cliente HTTP real (`reqwest`).
- Migrations aplicadas no contêiner como em produção (mesmos papéis por módulo, RLS ligado e
  forçado). Teste que precisa de papel administrativo para preparar dados usa uma conexão separada
  só na preparação; a ação testada sempre passa pela rota.
- Provedores externos simulados por servidor fake (`wiremock`) que assina os eventos como o
  provedor real. Fakes: PSP do PIX, Meta (WhatsApp), prefeitura (NFS-e), provedor de assinatura,
  Memed, operadoras (webservice TISS e elegibilidade).
- Relógio injetado (validade de senha, expiração de reserva, vínculo de cuidado de 30 dias).
- Isolamento: um banco por suíte ou por teste (template database), nunca estado compartilhado
  entre testes. Nunca URL de produção.

## 2. Exemplo: teste de rota com Testcontainers

Pseudocódigo próximo do real (nomes de helpers a definir no esqueleto da Fase 0):

```rust
// pseudocódigo
#[tokio::test]
async fn fc_cxa_02_total_enviado_pela_tela_e_ignorado() {
    let app = AppDeTeste::subir().await;               // PostgreSQL em contêiner + axum em porta aleatória
    let carla = app.usuario("Carla", Perfil::Caixa).com_caixa_aberto().await;
    let sonia = app.paciente("Sônia Maria Castro", "1962-02-03").await;
    app.tabela_particular(12, &[("ecocardiograma", 38000)]).await;

    let resp = app.post("/v1/caixa/recebimentos")
        .como(&carla)
        .idempotency_key(Uuid::now_v7())
        .json(&json!({ "paciente_id": sonia.id, "itens": [{ "exame": "ecocardiograma", "total": 10000 }] }))
        .enviar().await;

    assert_eq!(resp.status(), 201);
    let rec = app.db.recebimento(resp.id()).await;
    assert_eq!(rec.total_centavos, 38000);              // o servidor recalcula (G1)
    assert_eq!(rec.versao_tabela, 12);                  // regra aplicada no lastro (docs/21)
}
```

O que torna o teste válido: rota real, banco real, valor esperado vindo da frase (R$ 380,00), efeito
conferido no banco e não só o status.

## 3. Matriz por fluxo

Todo fluxo tem pelo menos uma linha em cada classe. Exemplo, Caixa do particular (FC-CXA):

| Classe | Casos |
|---|---|
| Feliz | FC-CXA-01 total do servidor; FC-CXA-03 desconto dentro do limite |
| Erro esperado | FC-CXA-04 desconto acima do limite (403); FC-CXA-08 sem caixa aberto (409) |
| Repetição | FC-CXA-06 mesma `Idempotency-Key` (uma cobrança PIX); duas requisições simultâneas |
| Limite | FC-CXA-07 valor zero ou negativo (422); desconto de 100% |
| Acesso indevido | FC-CXA-09 quem pede estorno não aprova (403); operador de outro caixa |

Use esta forma para registrar a matriz de cada módulo no plano de testes.

## 4. Fluxos críticos a cobrir

Do `docs/15` G4 (12 fluxos): login, 2FA e troca de perfil; cadastro de paciente e convite do
portal; agendamento, encaixe e lista de espera; check-in; caixa (abertura, recebimento, desconto,
estorno, sangria, fechamento); webhook do PIX; worklist, MPPS, recebimento de imagem e conciliação;
laudo (rascunho, assinatura, retificação, publicação); faturamento (guia, lote, demonstrativo,
glosa); estoque (baixa por exame); portal (paciente só vê o que é seu); webhooks do WhatsApp e da
NFS-e.

Do `docs/27` (10 conjuntos de frases): FC-AGD agendamento com guia; check-in com dois
identificadores; bipe da pulseira; triagem com contraste bloqueante; laudo assinado, portal e
correção; FC-CXA caixa; FC-PIX webhook; FC-ETQ etiqueta reimpressa; FC-ACE acesso indevido;
FC-CTG contingência.

Ordem sugerida de construção, acompanhando as fases do `docs/25` § 3.1: Fase 1 identidade,
agenda com guia, check-in, etiquetas, caixa, PIX, contingência, convite; Fase 2 worklist, MPPS,
conciliação, triagem, laudo, portal; Fase 3 faturamento, estoque.

## 5. Fluxos com Orthanc e NATS

- Worklist: criar pedido pela rota e consultar o MWL do Orthanc em contêiner (C-FIND com
  ferramenta DICOM de teste) conferindo nome, nascimento e accession.
- Imagem: enviar DICOM de exemplo por C-STORE ao Orthanc de teste e esperar o evento
  `EstudoRecebido` no NATS; conferir hash gravado e conciliação. Estudo sem conciliação não
  aparece na fila de laudo (REQ-EXA-001, RC-002).
- Esperar evento por assinatura com prazo máximo, nunca `sleep`.
- AE Title não cadastrado é recusado (SEC-MSG-7).

## 6. Lastro e eventos (`docs/21` § 5 item 9)

Para cada fluxo crítico, o teste confere:
1. o registro tem os metadados obrigatórios (`criado_por`, `origem`, `estacao`, `unidade`,
   `versao`, `correlacao` com o `trace_id`);
2. o evento esperado foi gravado na tabela append-only do domínio, com o hash encadeado válido;
3. as ligações tipadas existem (agendamento para guia, atendimento para etiqueta, laudo para
   assinatura);
4. repetir a ação não cria segundo evento nem segunda ligação;
5. correção grava nova versão com `motivo` e mantém a anterior.

Teste de schema (CI): nenhuma tabela sem as colunas padrão do `docs/21` § 1.

## 7. RLS (`docs/18` § 7)

Para cada política, dois testes direto no banco e dois pela rota:

```rust
// pseudocódigo
#[tokio::test]
async fn rls_laudo_leitura_medico_sem_vinculo_nao_ve() {      // FC-ACE-05
    let app = AppDeTeste::subir().await;
    let laudo_b = app.laudo_assinado_de("Antônio Carlos Ribeiro", "1954-06-15").await;
    let marcos = app.usuario("Dr. Marcos Freire", Perfil::Medico).await; // sem vínculo com B

    let mut tx = app.db.papel("laudos_app").begin().await;
    tx.set_local("app.usuario_id", marcos.id).await;
    tx.set_local("app.perfis", "medico").await;
    tx.set_local("app.unidade_id", app.unidade_centro()).await;
    tx.set_local("app.break_glass", "false").await;

    let linhas = tx.query("SELECT id FROM laudos.laudo WHERE paciente_id = $1", laudo_b.paciente_id).await;
    assert!(linhas.is_empty());                               // o banco barra mesmo se a rota errar
}

#[tokio::test]
async fn rls_laudo_leitura_medico_com_vinculo_ve() {
    // mesmo preparo, com vínculo de cuidado válido (agendamento com ele) e expira_em no futuro
    // assert_eq!(linhas.len(), 1);
}
```

Regras: teste roda com o papel do módulo (sem `BYPASSRLS`); contexto por `SET LOCAL` dentro da
transação; um teste confere que contexto de uma requisição não vaza para a próxima no pool;
vínculo expirado (31 dias, relógio injetado) não vê.

## 8. Contingência (`docs/23` § 13)

Os 15 testes da tabela são obrigatórios e seguem esta skill: reserva de faixa simultânea, lançamento
repetido e simultâneo, número fora da faixa (422), marca `origem = contingencia`, preço do servidor,
duplicado sem união automática, estudo `C-NNNN` fora da fila de laudo, conciliação sem dois
identificadores recusada, encerramento com pendência recusado, declaração offline uma vez só, cópia
local expirada, acesso indevido, lançamento após encerramento recusado.

## 9. O que reprova um teste G4 na revisão

- usa mock do repositório, do banco ou do serviço de domínio;
- chama a função do domínio em vez da rota;
- prepara o estado e testa a ação pelo mesmo atalho administrativo;
- confere só o status HTTP;
- simula concorrência em sequência;
- depende de outro teste ter rodado antes.
