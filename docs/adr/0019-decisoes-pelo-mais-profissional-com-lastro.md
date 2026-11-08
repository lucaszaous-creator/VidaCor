# ADR-0019 · Toda decisão escolhe o caminho mais profissional e rastreável, e é registrada

- **Status:** aceito (regra do dono do produto, 08/11/2026)
- **Data:** 2026-11-08

## Contexto
O dono do produto determinou: "resolva sempre partindo para o lado mais profissional e lastrável, sempre com lastro.
Isso é regra do projeto." Até aqui, várias pendências ficavam esperando escolha, e decisões menores não tinham
registro de quem decidiu e por quê.

## Decisão
1. Quando houver mais de um caminho, quem trabalha no projeto (pessoa ou IA) escolhe o **mais profissional**
   (padrão de mercado, norma ou prática de clínicas grandes) e o **mais rastreável** (deixa lastro: quem, quando,
   por quê, versão anterior).
2. Toda decisão tomada assim entra no **registro de decisões** [`docs/decisoes.md`](../decisoes.md), com data,
   quem decidiu, alternativas, motivo, onde foi aplicada (arquivos/commit) e como desfazer.
3. Nada é apagado para resolver: substituição deixa a versão anterior no histórico (git, versão do documento ou
   evento de lastro) e o motivo.
4. Só sobe para o dono do produto o que depende de informação que só ele tem (dinheiro, contrato, pessoas,
   prazos) ou de aprovação formal exigida por regra (frases de comportamento, responsável clínico de segurança).
   Mesmo nesses casos, entrega-se uma proposta pronta com premissas marcadas "a confirmar".

## Consequências
Menos espera por decisões menores; histórico completo de cada escolha; o dono do produto revisa o registro e
pode reverter qualquer item pelo caminho indicado.
