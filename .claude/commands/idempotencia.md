---
description: Idempotência de webhooks e pagamentos
---

Você é um engenheiro sênior especialista em pagamentos e integrações. Quero que todo webhook e todo endpoint de cobrança ou de efeito externo do VidaCor aceite repetição sem efeito duplicado, seguindo o G3 do CLAUDE.md.

1. Liste todos os webhooks recebidos (PSP do PIX, Meta/WhatsApp, NFS-e, provedor de assinatura) e todos os endpoints que cobram, estornam, emitem nota, enviam lote TISS, assinam ou publicam laudo, ou mandam mensagem transacional ao paciente.
2. Webhooks:
   a. confirme que a assinatura é verificada com o corpo bruto da requisição antes de qualquer processamento (no axum, extrair `Bytes` antes de desserializar);
   b. use/crie a tabela de eventos recebidos com (provedor, evento_id) como chave única;
   c. ao receber um evento, tente gravar o ID primeiro (INSERT … ON CONFLICT DO NOTHING); se já existir, responda 200 e não faça mais nada;
   d. não dependa da ordem de chegada: quando precisar do estado atual, busque o objeto na API do provedor;
   e. responda 2xx rápido e mande o trabalho pesado para o outbox/NATS.
3. Chamadas que fazemos aos provedores (cobrança PIX, estorno, NFS-e, lote TISS, assinatura, WhatsApp): envie uma chave de idempotência (UUID) gerada uma vez por operação de negócio, gravada no banco e reaproveitada em caso de nova tentativa.
4. Rotas do núcleo que criam recebimento, estorno, agendamento, guia, lote ou assinatura: aceite o header Idempotency-Key, guarde a chave com o hash do corpo e a resposta, devolva a mesma resposta se a chave se repetir e 409 se o corpo for diferente.
5. Escreva testes que enviam o mesmo evento 2 vezes, e 2 vezes ao mesmo tempo, e provam que o efeito acontece só uma vez.

Regras: mostre o plano antes de alterar código. Não mude valores, preços nem regras de cobrança. Não use dado pessoal (CPF, e-mail, telefone, nome) como chave de idempotência.
