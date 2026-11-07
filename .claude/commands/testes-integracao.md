---
description: Testes de integração dos fluxos críticos
---

Você é um engenheiro de qualidade. Quero cobrir os fluxos críticos do VidaCor com testes de integração no estilo request spec: cada teste chama a rota HTTP de verdade do núcleo (axum), com PostgreSQL de teste de verdade (Testcontainers) e Orthanc/NATS reais quando o fluxo usar, e só simula o que é de fora (PSP do PIX, Meta/WhatsApp, prefeitura/NFS-e, provedor de assinatura, Memed). Regras de referência: CLAUDE.md (G4) e docs/15-padrao-de-qualidade.md.

1. Leia o projeto e liste os fluxos críticos: login e 2FA, cadastro de paciente e convite do portal, agendamento/encaixe, check-in, caixa (recebimento, desconto, estorno, sangria, fechamento), webhooks do PIX/WhatsApp/NFS-e, worklist/MPPS/imagem/conciliação, laudo (assinatura, retificação, publicação), faturamento TISS, estoque e qualquer rota que mexe com dinheiro, permissão, dado clínico ou dado de outro paciente.
2. Para cada fluxo, escreva em português as frases de comportamento no formato describe / it ("quando..., então..."), cobrindo:
   a. o caminho feliz;
   b. erros esperados (pagamento recusado, campo inválido, usuário sem permissão, convênio inelegível);
   c. repetição (clicar 2 vezes, webhook duplicado, 2 requisições ao mesmo tempo);
   d. limites (horário já ocupado, estoque zerado, caixa fechado, valor zero ou negativo, convite expirado);
   e. acesso indevido (usuário ou paciente A tentando ler ou alterar dado de B; perfil sem permissão para o ato).
3. Me mostre as frases e espere eu confirmar ou corrigir. Eu sou a fonte da verdade sobre o comportamento esperado.
4. Configure o ambiente de teste com a ferramenta que o projeto já tem (cargo nextest + Testcontainers; Vitest/Playwright na tela). Se faltar algo, sugira o mais simples para a stack e espere meu OK. O banco de teste é sempre um contêiner descartável. Nunca aponte para o banco de produção nem use dado real de paciente.
5. Escreva os testes batendo nas rotas reais. Mock só de serviço externo (servidor fake com wiremock, com eventos assinados); banco, regras de negócio e permissões rodam de verdade. Para repetição simultânea, dispare as requisições ao mesmo tempo (tokio::join! / futures::future::join_all).
6. Opcional, só se eu aprovar: um único teste E2E de fumaça (Playwright) para o caminho agendamento → check-in → caixa → exame → laudo assinado.
7. Rode tudo e me diga o que passa e o que falha. Teste que falha por bug real fica falhando e entra em docs/bugs.md; não ajuste o teste para passar.
8. Confirme o comando único para rodar todos os testes (`just test`), para rodar depois de toda alteração feita por IA.

Regras: é proibido apagar, pular (#[ignore], skip) ou afrouxar um teste para fazê-lo passar. Não mocke o banco nem a regra que está sendo testada. Se um teste parecer errado, pare e me pergunte.
