# ADR-0013 · Distribuição por instaladores e atualização pelo servidor local

- **Status:** proposto
- **Data:** 2026-11-07

## Contexto
O sistema precisa ser instalável nas máquinas da clínica (servidor e estações), funcionar sem internet nas estações e ser atualizado com segurança.

## Decisão
Instalador do servidor (Linux recomendado; Windows Server como alternativa) com provisionamento reproduzível; estações com VidaCor Desktop (Tauri) distribuído como MSI assinado, com perfis por máquina; atualizações assinadas baixadas pelo servidor e servidas às estações pela rede local; canais estável e piloto; rollback. Detalhes em [13](../13-instalacao-e-distribuicao.md).

## Consequências
Instalação e atualização padronizadas e auditáveis; necessidade de certificado de assinatura de código e de manter o repositório de atualizações no servidor.
