# 15 · Padrão de qualidade: as seis garantias

> **Status:** proposta (v0.1, nov/2026). Decisão no
> [ADR-0014](adr/0014-garantias-no-servidor-e-no-banco.md). Resumo obrigatório em
> [CLAUDE.md](../CLAUDE.md). Os roteiros de auditoria e correção estão em
> [playbooks/](playbooks/).

Este padrão existe para que nenhum código do VidaCor, escrito por pessoas ou com
ajuda de IA, entre em produção com as falhas clássicas: **regra no cliente,
condição de corrida, efeito duplicado, teste de mentira e arquivo que faz tudo.**

---

## G1 · O servidor decide; o cliente só pede

### Onde se aplica no VidaCor
| Regra | Onde mora | O cliente pode… |
|---|---|---|
| Preço do exame particular, desconto, total do caixa | `dominio-caixa` | só enviar itens e forma de pagamento; o total é recalculado |
| Desconto acima do limite | `dominio-caixa` + autorização do gerente no servidor | pedir; quem aprova é o servidor |
| Valor e códigos da guia TISS | `dominio-faturamento` | nada; é gerado no servidor |
| Cortesia e cobrança do estacionamento | `dominio-estacionamento` | nada |
| Permissão por ato (ver laudo, estornar, assinar) | `identidade` + checagem em cada rota | nada; o botão escondido é só UX |
| Publicação do laudo no portal | `dominio-laudos` | nada; só laudo assinado e liberado |

### Segredos
| Tipo | Exemplo | Pode ir para a tela? |
|---|---|---|
| Público por design | URL do servidor, ID público do cliente OIDC, chave pública de verificação | Sim |
| Secreto | Credencial do PSP do PIX, token da Meta, certificado/senha de assinatura, credencial da NFS-e, senha do banco, chave de API de IA | **Nunca** |

Segredo publicado por engano é **rotacionado** imediatamente e registrado em incidente.

### Banco e portal
O núcleo é o único que fala com o PostgreSQL, com **um papel por módulo** (menor
privilégio). O portal, na nuvem, usa **RLS**: cada paciente lê só as próprias
linhas, e campos sensíveis (status de pagamento, papel, publicação) só mudam pelo
servidor. Política `USING (true)` / `WITH CHECK (true)` é proibida.

---

## G2 · Concorrência se resolve no banco

### Recursos limitados do VidaCor e a proteção prevista
| Recurso | Se 2 pedidos chegam juntos, sem proteção… | Proteção no banco |
|---|---|---|
| **Horário de sala/equipamento** | Dois pacientes no mesmo horário | Restrição de exclusão (`EXCLUDE USING gist` com `tstzrange`) por sala |
| **Encaixe na lista de espera** | Duas recepcionistas dão o mesmo horário | Mesma restrição + `UPDATE … WHERE status = 'livre'` |
| **Senha da fila** | Duas senhas com o mesmo número | Sequência por dia/tipo ou índice único `(data, tipo, numero)` |
| **Chamada da próxima senha** | Dois guichês chamam o mesmo paciente | `UPDATE … SET status='chamado' … WHERE status='aguardando'` conferindo linhas afetadas, ou `FOR UPDATE SKIP LOCKED` |
| **Caixa aberto** | Dois caixas abertos para o mesmo operador | Índice único parcial `(operador) WHERE fechado_em IS NULL` |
| **Recebimento** | Cobrança duplicada no clique duplo | `Idempotency-Key` + índice único |
| **Baixa de estoque (contraste)** | Estoque negativo | `UPDATE lote SET qtd = qtd - :n WHERE id = :id AND qtd >= :n` |
| **Número de guia / lote TISS** | Número repetido | Sequência por convênio + índice único |
| **Link de convite do portal** | Link usado duas vezes | `UPDATE convite SET usado_em = now() WHERE token = :t AND usado_em IS NULL` |
| **Conciliação paciente × estudo** | Estudo ligado a dois pacientes | Índice único no `study_instance_uid` |
| **Assinatura do laudo** | Duas versões "finais" | Índice único parcial `(laudo_id) WHERE vigente` + concorrência otimista |
| **Cortesia do estacionamento** | Cortesia aplicada duas vezes | Índice único `(passagem_id)` na tabela de cortesia |

### Regras
- Ordem de preferência: **constraint única → operação atômica → `FOR UPDATE`**.
- Botão desabilitado durante o envio é **só conforto de UX**.
- Lock em memória, Redis com expiração ou cache **não** contam como proteção.
- Teste obrigatório: duas requisições **simultâneas** (`tokio::join!` / `futures::join_all`), prova de que só uma teve efeito.

---

## G3 · Repetir não duplica

### Webhooks que o VidaCor recebe
| Origem | Evento | Proteção |
|---|---|---|
| PSP do PIX | Pagamento confirmado / devolvido | Assinatura com corpo bruto, `eventos_recebidos(provedor, evento_id)` único, inserir antes de agir |
| Meta (WhatsApp) | Mensagem recebida, status de entrega | Idem; status fora de ordem tratado por "estado mais avançado vence" |
| Provedor de NFS-e | Nota autorizada / rejeitada | Idem; estado atual buscado na API do provedor |
| Provedor de assinatura em nuvem | Assinatura concluída | Idem |

Fluxo padrão: **verificar assinatura → inserir ID (se conflito, 200 e fim) →
gravar no outbox → 200** e o processamento pesado segue assíncrono.

### Chamadas que o VidaCor faz
| Operação | Chave |
|---|---|
| Criar cobrança PIX, estornar | UUID por `recebimento_id`/`estorno_id`, guardado no banco e reaproveitado |
| Emitir NFS-e | UUID por recebimento |
| Enviar lote TISS | UUID por lote |
| Assinar laudo | UUID por versão do laudo |
| Enviar WhatsApp transacional | UUID por (mensagem, destinatário interno) |

### Rotas do núcleo com `Idempotency-Key`
Recebimento no caixa, estorno, sangria/suprimento, criação de agendamento,
check-in, geração de guia, envio de lote, assinatura e liberação de laudo. A
chave é guardada com o hash do corpo e a resposta; repetição com o mesmo corpo
devolve a mesma resposta, e com corpo diferente devolve `409`.

**Nunca** CPF, e-mail, telefone ou nome como chave.

---

## G4 · Teste de integração de verdade

### Fluxos críticos do VidaCor
1. Login, 2FA e troca de perfil
2. Cadastro de paciente e convite do portal
3. Agendamento, encaixe e lista de espera
4. Check-in (identidade, elegibilidade, guia, documentos)
5. Caixa: abertura, recebimento, desconto, estorno, sangria, fechamento
6. Webhook do PIX
7. Worklist, MPPS, recebimento de imagem e conciliação
8. Laudo: rascunho, assinatura, retificação e publicação no portal
9. Faturamento: guia, lote, demonstrativo e glosa
10. Estoque: baixa por exame
11. Portal: paciente só vê o que é seu
12. Webhooks do WhatsApp e da NFS-e

### Formato das frases (aprovadas antes de escrever o teste)
```
describe "Caixa · recebimento particular"
  it "quando o operador recebe R$ 180 pelo PIX e o PSP confirma, então o recebimento fica pago e a NFS-e é solicitada uma vez"
  it "quando o cliente envia um total diferente do calculado, então o servidor ignora e usa o total recalculado"
  it "quando o mesmo recebimento é enviado duas vezes com a mesma Idempotency-Key, então só um recebimento existe"
  it "quando duas requisições de recebimento chegam ao mesmo tempo, então só uma cobrança é criada"
  it "quando o desconto passa do limite sem autorização do gerente, então a rota responde 403"
  it "quando o valor é zero ou negativo, então a rota responde 422"
  it "quando um operador tenta estornar recebimento de outro caixa sem permissão, então a rota responde 403"
```

### Ambiente
- Rust: `cargo nextest` + **Testcontainers** (PostgreSQL, Orthanc, NATS); servidor axum subido em porta aleatória; cliente HTTP real.
- Provedores externos simulados por servidor fake (`wiremock`) com eventos assinados.
- Comando único: **`just test`**. E2E de fumaça (Playwright) só nas jornadas aprovadas.
- Teste instável é bug. Teste falhando por bug real fica falhando e vai para [bugs.md](bugs.md).

---

## G5 · Camadas separadas

```
apps/web (tela)  →  crates/api (rotas, autenticação, serialização)
                        →  crates/dominio-* (regras, sem I/O)
                        →  crates/*-adaptador (PostgreSQL, Orthanc, PIX, WhatsApp, NFS-e)
```
- A tela só chama o cliente gerado do OpenAPI.
- Rota HTTP: valida entrada, chama o caso de uso, devolve resposta. Sem regra de negócio.
- Domínio: sem `sqlx`, `reqwest`, `axum` **[CI: teste de dependências entre crates]**.
- Limites: 400 linhas (revisão), 600 (justificar ou dividir) **[CI: aviso]**.

---

## G6 · Plano antes, alteração depois

| Situação | Regra |
|---|---|
| Auditoria | Somente leitura; relatório em `AUDITORIA.md` no modelo de [playbooks/modelo-auditoria.md](playbooks/modelo-auditoria.md) |
| Correção | Plano → OK do dono → um passo por vez → testes a cada passo |
| Migration | Revisada por uma pessoa; aditiva; nunca apaga dado sozinha |
| Proteção existente | Nunca desligada para "fazer funcionar" |
| Dúvida sobre comportamento | O dono do produto é a fonte da verdade: perguntar |
| Não deu para verificar | Escrever "não verificado" e o motivo |

---

## Checklist do pull request

- [ ] Regra de preço, permissão ou limite está no servidor e é recalculada (G1)
- [ ] Nenhum segredo na tela ou em variável do navegador (G1)
- [ ] Todo "ler, decidir, gravar" protegido por constraint, operação atômica ou `FOR UPDATE`, com teste simultâneo (G2)
- [ ] Webhook verifica corpo bruto e grava ID único antes de agir; chamadas externas com chave de idempotência; rota de escrita aceita `Idempotency-Key` (G3)
- [ ] Testes de integração na rota real cobrindo feliz, erro, repetição, limite e acesso indevido; nenhum teste pulado ou afrouxado (G4)
- [ ] Tela sem regra, domínio sem I/O, arquivos dentro do limite (G5)
- [ ] Migration revisada; nada apagado; plano aprovado quando exigido (G6)
- [ ] `just test` verde (ou falhas de bug real listadas em `docs/bugs.md`)
