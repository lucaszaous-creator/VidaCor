# Playbook · Condições de corrida

> Uso: `/condicao-de-corrida`. Regras em [CLAUDE.md](../../CLAUDE.md) e [docs/15](../15-padrao-de-qualidade.md).

```text
Você é um engenheiro sênior revisando o VidaCor. Quero achar e corrigir condições de corrida, seguindo o G2 do CLAUDE.md.

1. Varra o código e liste todo lugar que faz "ler, decidir e depois gravar": agendamento e encaixe, senha da fila e chamada de senha, abertura de caixa, recebimento, baixa de estoque, número de guia e de lote, convite do portal, conciliação de estudo, versão de laudo, cortesia do estacionamento, contadores e qualquer recurso limitado.
2. Para cada ocorrência, explique em uma frase o que acontece se 2 requisições iguais chegarem no mesmo milissegundo (ex.: dois pacientes no mesmo horário, estoque negativo, cobrança duplicada, duas versões vigentes do laudo).
3. Proponha a correção mais simples que resolve no PostgreSQL, nesta ordem de preferência:
   a. constraint única ou de exclusão (unique index, índice único parcial, EXCLUDE USING gist para faixas de horário) que impede o registro duplicado;
   b. operação atômica no próprio banco (ex.: UPDATE lote SET qtd = qtd - 1 WHERE id = X AND qtd > 0, conferindo as linhas afetadas);
   c. transação com lock da linha (SELECT … FOR UPDATE, ou FOR UPDATE SKIP LOCKED para filas) quando a regra for mais complexa.
4. Na tela, desabilite o botão durante o envio, mas trate isso como conforto de UX. A proteção de verdade fica no servidor e no banco.
5. Para cada correção, escreva um teste que dispara 2 requisições simultâneas (tokio::join!) e prova que só uma tem efeito.

Regras: não proponha Redis, fila, lock em memória ou qualquer infraestrutura nova se o banco resolve; se achar que precisa, explique por que o banco não basta antes. Mostre a lista completa e o plano antes de alterar qualquer arquivo. Toda migration nova deve ser revisada por mim antes de rodar. Se já existirem dados duplicados que impeçam criar a constraint, avise e proponha como limpar, sem apagar nada sozinho.
```
