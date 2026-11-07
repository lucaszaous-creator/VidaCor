# ADR-0017 · Permissões em três camadas (perfil, contexto e RLS)

- **Status:** proposto
- **Data:** 2026-11-07

## Contexto
O VidaCor guarda dado de saúde sensível e é usado por perfis muito diferentes (recepção, técnico, médico, faturamento, financeiro, TI, paciente). Um erro de permissão no código não pode virar vazamento.

## Decisão
Permissão em três camadas: perfil (RBAC, catálogo de atos), contexto (ABAC: unidade, sala, vínculo de cuidado, laudo atribuído, caixa próprio) decidido no núcleo, e RLS forçado no PostgreSQL como segunda barreira, com o contexto declarado por transação (`SET LOCAL`). Vínculo de cuidado com expiração, acesso de emergência com justificativa e revisão, separação de funções e TI sem dado clínico. Detalhes e matriz em [docs/18](../18-perfis-permissoes-e-rls.md).

## Consequências
Mais modelagem e testes de política desde o início; um bug de autorização no código não basta para expor dado clínico. As políticas precisam de testes próprios e revisão a cada migration.
