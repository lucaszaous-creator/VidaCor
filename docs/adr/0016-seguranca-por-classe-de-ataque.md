# ADR-0016 · Segurança por classe de ataque (Cloudflare security-audit)

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-07

## Contexto
O time quer regras de segurança de nível profissional que guiem a escrita do código, e não só auditorias depois. A Cloudflare publicou (licença MIT) a skill `security-audit`, com metodologia de auditoria e um catálogo de classes de ataque por domínio (web e autenticação, navegador, desktop, mensageria, disponibilidade, isolamento de dados, cadeia de suprimentos, infraestrutura, IA, memória).

## Decisão
Instalar a skill em `.claude/skills/security-audit/` (com licença e commit de origem) e adotar o [docs/17](../17-seguranca-por-classe-de-ataque.md): Parte A define quando e como auditar (modos, princípios, veredictos, gravidade, segurança da execução, anti-padrões); Parte B transforma cada classe de ataque em regras de construção `SEC-*` aplicadas ao VidaCor.

## Consequências
Cada PR passa a declarar os domínios de segurança tocados; várias regras viram verificações de CI desde o esqueleto. A skill é de terceiros: atualizações seguem revisão como qualquer dependência.
