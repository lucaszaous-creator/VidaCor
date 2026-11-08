# ADRs — decisões de arquitetura

| ADR | Decisão | Status |
|---|---|---|
| [0001](0001-registrar-decisoes-em-adr.md) | Registrar decisões de arquitetura em ADR | Proposto |
| [0002](0002-rust-no-nucleo.md) | Rust no núcleo do sistema | Proposto |
| [0003](0003-poliglota-por-componente.md) | Poliglota por componente, com contratos explícitos | Proposto |
| [0004](0004-monolito-modular.md) | Monólito modular em vez de microserviços | Proposto |
| [0005](0005-frontend-typescript-react.md) | Frontend em TypeScript + React, casca desktop em Tauri | Proposto |
| [0006](0006-pacs-orthanc-ohif.md) | PACS com Orthanc e visualização com OHIF | Proposto |
| [0007](0007-mensageria-outbox-nats.md) | Mensageria com outbox transacional e NATS JetStream | Proposto |
| [0008](0008-assinatura-icp-servico-jvm.md) | Assinatura ICP-Brasil em serviço JVM dedicado | Proposto |
| [0009](0009-postgresql.md) | PostgreSQL como banco principal | Proposto |
| [0010](0010-identidade-keycloak.md) | Identidade com Keycloak (OIDC) | Proposto |
| [0011](0011-portal-fora-do-servidor-local.md) | Portal do paciente fora do servidor local | Proposto |

| [0012](0012-observabilidade-opentelemetry-slo.md) | Observabilidade com OpenTelemetry, histogramas e SLOs | Proposto |
| [0013](0013-distribuicao-instaladores.md) | Distribuição por instaladores e atualização pelo servidor local | Proposto |
| [0014](0014-garantias-no-servidor-e-no-banco.md) | As seis garantias: servidor decide, banco protege, repetição não duplica | Proposto |
| [0015](0015-protecao-do-usuario.md) | Proteção do usuário: a cerca e a rede de segurança | Proposto |
| [0016](0016-seguranca-por-classe-de-ataque.md) | Segurança por classe de ataque (Cloudflare security-audit) | Proposto |
| [0017](0017-permissoes-e-rls.md) | Permissões em três camadas (perfil, contexto e RLS) | Proposto |
| [0018](0018-lastro-e-rastreabilidade.md) | Lastro e rastreabilidade em todo registro | Proposto |
| [0019](0019-decisoes-pelo-mais-profissional-com-lastro.md) | Decidir pelo mais profissional e rastreável, com registro | Aceito |

Visão geral em [11 · Arquitetura e infraestrutura](../11-arquitetura-e-infraestrutura.md).
