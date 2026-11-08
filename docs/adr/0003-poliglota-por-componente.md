# ADR-0003 · Poliglota por componente, com contratos explícitos

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-06

## Contexto
Nenhuma linguagem tem o melhor ecossistema para tudo: PACS (C++), assinatura ICP-Brasil (JVM), IA e visão computacional (Python), interface (TypeScript).

## Decisão
Cada componente usa a tecnologia mais madura para o seu problema. Comunicação somente por HTTP/gRPC versionado ou eventos no barramento; nenhum serviço acessa o banco de outro. Linguagens permitidas: Rust, TypeScript, Python, Kotlin/Java e SQL. Uma linguagem nova exige novo ADR.

## Consequências
Melhor ferramenta para cada problema; custo de manter mais de uma cadeia de build, controlado pelo monorepo e pelo executor único (just).
