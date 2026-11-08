# ADR-0011 · Portal do paciente fora do servidor local

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-06

## Contexto
O paciente acessa pela internet; expor o servidor da clínica aumentaria a superfície de ataque.

## Decisão
Portal hospedado em nuvem no Brasil, com banco próprio que recebe do servidor local somente o conteúdo publicado (laudos assinados, documentos, agendamentos), por canal de saída autenticado. O servidor local não aceita conexões de entrada da internet.

## Consequências
Menor risco para os dados clínicos; o portal continua no ar mesmo se a clínica ficar sem internet; requer sincronização e custo de hospedagem.
