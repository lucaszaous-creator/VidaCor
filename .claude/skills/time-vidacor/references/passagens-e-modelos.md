# Passagens entre papéis e modelos de delegação

## Regra geral

Quem passa o trabalho escreve a nota. Quem recebe confere a nota contra o portão anterior
(`references/fluxo-de-entrega.md`) e devolve com a lista do que falta, se faltar algo. Nada é
assumido de memória: o que não está em artefato com caminho não foi passado.

## Nota de passagem (modelo único)

```markdown
## Passagem: <papel de origem> para <papel de destino>

- Item: <ID do épico ou requisito, ex.: REQ-AGD-012>
- Portão atingido: <P1 a P9>
- Feito: <uma a três frases>
- Artefatos:
  - `<caminho>` (<o que é>)
- Decisões tomadas: <DEC-AAAA-MM-DD-NN ou "nenhuma">
- A confirmar: <premissas ou dúvidas, cada uma com quem responde>
- Para o próximo papel:
  1. <ação>
  2. <ação>
- Pronto quando: <critério verificável>
- Não fazer: <limites, ex.: "não alterar a frase FC-AGD-03, aguarda nova aprovação">
```

## O que cada passagem precisa levar

| De | Para | Obrigatório na nota |
|---|---|---|
| produto | ux-ui | Problema, perfis e momento do dia, requisitos com ID, tarefas críticas suspeitas, mockups existentes relacionados |
| ux-ui | produto | Telas alteradas (`design/mockups/*.dc.html`), estados desenhados, mensagens (R8), pontos que mudam comportamento e precisam de decisão |
| produto | qa | Requisitos e frases FC-* com status, critérios do `docs/25` § 3.3 do módulo |
| qa | produto | Lacunas de cobertura nas frases (falta repetição, limite ou acesso indevido), perguntas de comportamento |
| produto e ux-ui | desenvolvimento | Design aprovado, frases aprovadas, especificação de tela (estados, ações vindas do servidor, atalhos), níveis de dado por perfil |
| desenvolvimento | qa | PR, rotas e eventos novos, migrations, garantias aplicadas, onde estão os testes, como rodar |
| desenvolvimento | devops-seguranca | Serviços, portas, variáveis (sem valor de segredo), métricas e alertas novos, domínios SEC-* tocados, necessidades de rede |
| qa | desenvolvimento | Teste que falha, frase que ele prova, evidência (saída do teste, trace), registro em `docs/bugs.md` |
| qa | devops-seguranca | Resultado de carga e caos, falhas de ambiente |
| devops-seguranca | produto | Prontidão, janela proposta, riscos de operação, comunicação para a equipe da clínica |

## Delegando para um agente do time

O agente começa sem a conversa. A mensagem precisa bastar.

```markdown
Objetivo: <o que precisa existir ao final>
Papel: <produto | ux-ui | dev-backend | dev-frontend | dev-mobile | qa | devops-seguranca>
Entradas:
- `<caminho>` (<o que é>)
Já decidido (não reabrir):
- <decisão> (<DEC-... ou documento>)
A confirmar (pode propor, não decidir):
- <dúvida>
Limites:
- Não fazer commit em main, não aprovar nem mesclar PR, nada em produção, só dado fictício.
- <limite específico>
Entrega esperada:
- <artefatos com caminho>
- Nota de passagem no modelo de `.claude/skills/time-vidacor/references/passagens-e-modelos.md`
```

## Devolvendo trabalho incompleto

```markdown
## Devolução: <papel> para <papel>

- Item: <ID>
- Portão que não foi atingido: <P?>
- Falta:
  1. <item do portão> (onde deveria estar: `<caminho>`)
- O que já dá para adiantar enquanto isso: <ou "nada">
```

## Exemplo

```markdown
## Passagem: produto para qa

- Item: REQ-AGD-012 (agendamento de convênio com guia)
- Portão atingido: P2
- Feito: frases FC-AGD-01 a FC-AGD-11 revisadas com o dono do produto.
- Artefatos:
  - `docs/27-frases-de-comportamento.md` § 1 (frases e status)
  - `docs/modelos/rastreabilidade.md` (linhas REQ-AGD-*)
- Decisões tomadas: DEC-AAAA-MM-DD-NN (prazo da reserva provisória: 48 h, a confirmar com a recepção)
- A confirmar: limite de encaixes por sala (responde: coordenação da recepção)
- Para o próximo papel:
  1. Montar o plano de testes do módulo a partir das frases aprovadas.
  2. Apontar frases sem caso de limite ou de acesso indevido.
- Pronto quando: cada FC-AGD tem caso de teste nomeado e ligado na matriz.
- Não fazer: não alterar texto de frase aprovada.
```
