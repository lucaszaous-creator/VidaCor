---
description: Refatoração segura em camadas
argument-hint: <caminho do arquivo>
---

Você é um engenheiro sênior especialista em refatoração segura. O arquivo $ARGUMENTS ficou grande demais e toda alteração quebra outra coisa. Quero dividir em camadas sem mudar comportamento, seguindo o G5 do CLAUDE.md e a estrutura da docs/11 (apps/ → crates/api → crates/dominio-* → adaptadores).

1. Mapeie o arquivo: liste cada função, componente, tipo e trecho, e classifique em uma destas camadas:
   a. UI (o que aparece na tela e captura evento do usuário, em apps/);
   b. regra de negócio (cálculos, validações, decisões; pertence a crates/dominio-*);
   c. acesso a dados e serviços externos (PostgreSQL/sqlx, Orthanc, PIX, WhatsApp, NFS-e, assinatura).
2. Aponte dependências cruzadas, código duplicado e estado global (statics, singletons, contexto global no React) que dificultam a separação.
3. Proponha a nova estrutura de pastas, módulos e crates, seguindo o padrão que o projeto já usa. Nada de framework ou biblioteca nova.
4. Monte um plano em passos pequenos. Cada passo move um pedaço só, mantém os mesmos nomes públicos (rotas, tipos exportados, props, colunas) e deixa o sistema compilando e funcionando.
5. Antes do primeiro passo, confira se existem testes cobrindo o comportamento atual. Se não existirem, escreva testes de caracterização (que registram o comportamento de hoje, mesmo que pareça errado) para os fluxos principais desse arquivo.
6. Execute um passo por vez, rodando `just test` ao final de cada um.

Regras: não corrija bugs nem mude regra de negócio durante a refatoração; se achar bug, anote em docs/bugs.md. Não renomeie rotas, props públicas, tipos do OpenAPI ou colunas do banco. Mostre o plano e espere meu OK antes do passo 1.
