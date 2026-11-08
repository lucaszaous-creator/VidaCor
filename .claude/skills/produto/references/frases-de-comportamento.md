# Frases de comportamento (FC-*)

Fontes: `docs/27-frases-de-comportamento.md`; `docs/15-padrao-de-qualidade.md` G4;
`docs/modelos/pacote-responsavel-clinico.md`; DEC-2026-11-08-34.

## 1. Para que servem

São o contrato entre a clínica e o time. Cada frase vira pelo menos um teste de integração
na rota HTTP real, com PostgreSQL real (Testcontainers) e Orthanc/NATS reais quando o fluxo
usar; mock só de serviço externo. As frases são aprovadas pelo dono do produto **antes** de
escrever o teste (G4). Defeito, no contrato, é comportamento diferente de uma frase aprovada
(`docs/25` §7.6).

## 2. Formato

Gherkin em português (`# language: pt`), uma `Funcionalidade` por fluxo, um `Contexto` com
os dados fictícios, um `Cenário` por frase com o ID no início do título.

```gherkin
# language: pt
Funcionalidade: <fluxo>

  Contexto:
    Dado <paciente fictício com nome completo e nascimento>
    E <dados do recurso: guia, sala, caixa, laudo>
    E <usuário fictício> com o perfil <perfil do docs/18>

  Cenário: FC-<MOD>-NN <título curto no presente>
    Quando <ação concreta, com valores>
    Então <resultado observável: código HTTP, estado, saldo, evento no lastro>
    E <mensagem ao usuário no formato R8 quando houver>
```

Convenções do `docs/27`:
- Dados fictícios dos mockups (`design/dados-ficticios.md`).
- "A rota responde" indica o código HTTP (409 conflito, 422 regra recusada, 403 sem permissão).
- Mensagem ao usuário segue R8: o que aconteceu, o que foi feito, o que fazer.
- Toda frase cita valores exatos (data, hora, sala, valor em reais), nunca "um valor qualquer".

## 3. IDs

`FC-<MOD>-NN`. Prefixos em uso: `AGD` agendamento com guia, `CHK` check-in, `PUL` pulseira,
`TRI` triagem, `LAU` laudo, `CXA` caixa, `PIX` webhook PIX, `ETQ` etiqueta, `ACS` acesso
indevido, `CTG` contingência (conferir a lista exata nas tabelas do `docs/27` antes de criar
um novo). Prefixo novo segue o prefixo do requisito do módulo.

## 4. Cobertura obrigatória por fluxo

Cada fluxo cobre as cinco classes (G4). Use esta tabela como lista de conferência:

| Classe | Pergunta | Exemplo no agendamento |
|---|---|---|
| Caminho feliz | O que acontece quando tudo está certo? | FC-AGD-01 guia válida consumida |
| Erro esperado | Quais recusas a regra prevê? | FC-AGD-02, 05, 06 |
| Repetição | Clique duplo, reenvio, webhook duplicado, duas requisições simultâneas | FC-AGD-03, 04, 11 |
| Limite | Zero, negativo, esgotado, expirado | FC-AGD-07, 08 |
| Acesso indevido | Perfil sem permissão; usuário ou paciente A no dado de B | FC-AGD-10 |

Fluxo com recurso limitado (G2) tem frase de **duas requisições simultâneas**. Escrita que
cria cobrança, recebimento, guia, agendamento ou laudo tem frase com **`Idempotency-Key`**
repetida (G3). Webhook tem frase de **evento duplicado** e de **ordem trocada**.

Frase de lastro: pelo menos uma por fluxo diz o que fica gravado (quem, canal, estação,
ligação), conforme `docs/21`.

## 5. Ciclo de aprovação

1. PO redige as frases e preenche a tabela da seção do fluxo com status "Aguardando aprovação".
2. Frases com efeito clínico (`CHK`, `PUL`, `TRI`, `LAU`, `CTG` e qualquer fluxo novo que toque identificação, exame, laudo, prescrição ou portal) vão também no pacote do responsável clínico.
3. O dono do produto marca cada frase: **Aprovada**, **Aprovada com ajuste** (ajuste em Observações) ou **Recusada** (com motivo), e preenche aprovador e data.
4. O assistente **nunca** preenche "Aprovada" nem o campo aprovador. Pode registrar uma aprovação que o dono deu por escrito, citando onde (mensagem, ata, comentário do PR).
5. Frase aprovada não muda. Mudança = nova linha com o mesmo ID e sufixo `v2`; a anterior fica registrada com status "Substituída por v2".
6. O teste que comprova cada frase é anotado na matriz de rastreabilidade pelo `qa`.

## 6. Tabela de status (modelo)

```markdown
| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-XXX-01 | <título> | Aguardando aprovação | | | |
| FC-XXX-01 v2 | <título ajustado> | Aguardando aprovação | | | Substitui FC-XXX-01 (motivo) |
```

## 7. Exemplo de fluxo novo, pronto para aprovação

```gherkin
# language: pt
Funcionalidade: Recebimento particular no caixa

  Contexto:
    Dado a paciente "Sônia Maria Castro", nascida em 03/02/1962, com atendimento particular aberto
    E o exame com valor de tabela de R$ 180,00 na regra de preço vigente
    E o caixa "Caixa 1" aberto pela operadora "Carla Mendes" com o perfil Caixa

  Cenário: FC-CXA-NN Total recalculado pelo servidor
    Quando a tela envia o recebimento com total de R$ 150,00
    Então o servidor ignora o total enviado e usa R$ 180,00
    E o recebimento grava a versão da regra de preço aplicada no lastro

  Cenário: FC-CXA-NN Recebimento repetido com a mesma chave
    Quando Carla envia o mesmo recebimento duas vezes com a mesma Idempotency-Key
    Então existe um só recebimento
    E as duas respostas são iguais
```

O nome da operadora do caixa no exemplo é ilustrativo; confira os nomes da equipe fictícia
em `design/dados-ficticios.md` antes de usar e substitua se houver outro definido. O número
`NN` só é atribuído na inclusão no `docs/27`, depois de conferir os IDs existentes.

## 8. Erros comuns

- Frase que descreve a tela ("clica no botão verde") em vez do comportamento.
- "Então o sistema funciona corretamente": resultado não observável.
- Esquecer a frase de acesso indevido ou a de repetição.
- Usar paciente real ou dado inventado fora da lista fictícia.
- Reescrever uma frase aprovada em vez de criar `v2`.
- Uma frase com duas regras: divida em dois cenários.
- Frase de tela usada como prova de regra de negócio: a prova é a integração (E2E-2).
