# ADR-0008 · Assinatura ICP-Brasil em serviço JVM dedicado

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-06

## Contexto
Laudos e documentos precisam de assinatura digital ICP-Brasil (PAdES), e o TISS pode exigir XAdES. O ecossistema Rust para esses padrões é imaturo.

## Decisão
Serviço em Kotlin com BouncyCastle e bibliotecas de assinatura ICP-Brasil, exposto por API interna; suporta certificado A1 (HSM/arquivo protegido) e certificado em nuvem do provedor.

## Consequências
Conformidade com bibliotecas maduras; isola o custo e a complexidade da assinatura atrás de um contrato simples.
