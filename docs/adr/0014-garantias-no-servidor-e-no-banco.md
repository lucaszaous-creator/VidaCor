# ADR-0014 · As seis garantias: servidor decide, banco protege, repetição não duplica

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-07

## Contexto
O VidaCor terá código escrito por pessoas e com apoio de IA. As falhas mais caras nesse cenário são previsíveis: regra de negócio no cliente, condição de corrida em recurso limitado, efeito duplicado em webhooks e pagamentos, testes que não exercitam o sistema real e arquivos que misturam camadas.

## Decisão
Adotar as seis garantias (G1–G6) do [CLAUDE.md](../../CLAUDE.md), detalhadas em [docs/15](../15-padrao-de-qualidade.md): regras no servidor; concorrência resolvida no PostgreSQL (constraint única → operação atômica → FOR UPDATE), sem locks em memória ou Redis; idempotência em webhooks, chamadas externas e rotas de escrita; testes de integração na rota real com banco real e mock só de terceiros; separação estrita de camadas; plano e revisão antes de alterar. Auditorias e correções seguem os playbooks de [docs/playbooks/](../playbooks/).

## Consequências
Mais trabalho de modelagem no banco e de testes desde o início; em troca, as classes de defeito mais comuns ficam bloqueadas por construção e verificáveis na CI e nas auditorias periódicas.
