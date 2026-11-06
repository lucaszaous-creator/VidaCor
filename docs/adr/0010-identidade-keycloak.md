# ADR-0010 · Identidade com Keycloak (OIDC)

- **Status:** proposto
- **Data:** 2026-11-06

## Contexto
2FA obrigatório, perfis por função, sessões no balcão, contas de paciente no portal e futura integração com apps.

## Decisão
Keycloak como provedor OIDC, com realm da equipe e realm de pacientes separados; o núcleo valida tokens e aplica as permissões por ato.

## Consequências
Recursos de segurança prontos (2FA, WebAuthn, políticas de senha, sessões); um serviço JVM a mais para operar.
