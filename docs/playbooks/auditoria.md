# Playbook · Auditoria geral (somente leitura)

> Uso: `/auditoria`. Regras em [CLAUDE.md](../../CLAUDE.md) e [docs/15](../15-padrao-de-qualidade.md).

```text
Você é um engenheiro sênior fazendo uma auditoria do sistema VidaCor. Nesta tarefa você NÃO altera nenhum arquivo do projeto, não roda migration e não instala nada. Você só lê o código, roda comandos de leitura e escreve um relatório. Regras de referência: CLAUDE.md e docs/15-padrao-de-qualidade.md.

Siga esta ordem e, ao final de cada etapa, anote os achados antes de passar para a próxima:

1. Testes de integração (G4)
   - Existe ferramenta de teste configurada (cargo nextest, Testcontainers, Vitest, Playwright)? Existe banco de teste separado do de produção? Qual o comando para rodar tudo (esperado: `just test`)? Rode (só se não tocar em produção) e diga quantos passam e quantos falham.
   - Liste os fluxos críticos do docs/15 (login e 2FA, cadastro e convite do portal, agendamento, check-in, caixa, webhook do PIX, worklist/imagem/conciliação, laudo e publicação, faturamento TISS, estoque, portal, webhooks do WhatsApp e da NFS-e) e diga quais têm teste batendo na rota HTTP de verdade, quais só têm teste unitário com mock e quais não têm nada.

2. Confiança no cliente (G1)
   - Regras de preço, desconto, cortesia, valor de guia, permissão, perfil ou limite que existem só em apps/web, apps/portal ou apps/desktop.
   - Chaves e tokens em código de tela ou em variáveis que vão para o navegador (VITE_*, configuração do Tauri, arquivos estáticos). Classifique como pública por design ou secreta.
   - Banco: o PostgreSQL está exposto a algo além do núcleo? Os papéis por módulo têm só o privilégio necessário? No portal, tabelas sem RLS e policies permissivas demais (USING (true), WITH CHECK (true), regras que não comparam com o paciente autenticado). Explique cada policy em português.

3. Condição de corrida (G2)
   - Todo trecho que lê, decide e depois grava sobre recurso limitado (horário de sala, encaixe, senha da fila, chamada de senha, caixa aberto, recebimento, lote de estoque, número de guia/lote, convite do portal, conciliação de estudo, versão de laudo, cortesia). Para cada um, diga o que acontece se 2 requisições iguais chegarem juntas e se existe constraint única, operação atômica ou lock de linha no banco protegendo.
   - Aponte proteção que depende de lock em memória, lock em Redis com expiração ou leitura de cache, e diga se o banco sozinho resolveria.

4. Idempotência (G3)
   - Webhooks recebidos (PSP do PIX, Meta/WhatsApp, NFS-e, assinatura): a assinatura é verificada com o corpo bruto? Existe registro de eventos processados com ID único? O que acontece se o mesmo evento chegar 2 vezes?
   - Chamadas aos provedores (cobrança e estorno PIX, NFS-e, lote TISS, assinatura, WhatsApp): usam chave de idempotência? Rotas de escrita do núcleo aceitam Idempotency-Key?

5. Separação de responsabilidades (G5)
   - Os arquivos mais longos do projeto e, em cada um, se mistura tela, regra de negócio e acesso a banco ou serviço externo; dependências proibidas entre crates. Aponte o candidato número 1 para refatorar.

Entrega: crie AUDITORIA.md na raiz seguindo docs/playbooks/modelo-auditoria.md, com uma tabela por etapa (achado, risco alto/médio/baixo, arquivo e linha, correção sugerida) e, no topo, os 5 achados de maior risco em ordem. Se não conseguir verificar algo, escreva "não verificado" e o motivo, sem chutar. Termine dizendo qual dos 5 playbooks de correção (/testes-integracao, /refatoracao-segura, /seguranca, /idempotencia, /condicao-de-corrida) eu devo rodar primeiro e por quê.
```
