# Teste de usabilidade com a equipe da clínica

Fonte: `docs/26-teste-de-usabilidade.md` (cumpre docs/16 C4 e o critério de aceite 5 do docs/25 § 3.2,
com base na IEC 62366-1). Este arquivo resume; o roteiro completo e as tabelas de tarefas estão no docs/26.

## 1. Rodadas

| Rodada | Quando | Material |
|---|---|---|
| R1 | Fase 0 | Mockups navegáveis (canvas, modo Play; `Fluxo-Completo.dc.html`) |
| R2 em diante | Antes do aceite de cada módulo | Homologação com dados fictícios |
| Final | Antes da virada | Homologação com fluxo completo |

## 2. Participantes e papéis

- Cinco participantes por perfil (recepção, técnico de sala ou enfermagem, médico, caixa,
  faturamento), da própria equipe, que fazem a tarefa no dia a dia.
- Quem desenhou as telas não participa como usuário. Mistura de tempo de casa e familiaridade.
- Moderador (não ajuda, não explica, pede para pensar em voz alta), anotador (formulário) e, nas
  sessões de técnico e médico, observador clínico (diz se o erro teria consequência clínica).
- Participantes identificados por código; consentimento assinado antes (USA-02, USA-04).

## 3. Preparação

1. Registrar a versão: commit do repositório ou SHA-256 dos `.dc.html` e endereço do canvas (USA-03).
2. Só dados fictícios (USA-05).
3. Estação parecida com a real: monitor de 1280×800 ou 1440×900, teclado, mouse, leitor de código de
   barras quando a tarefa pedir bipe.
4. Sala reservada, 45 a 60 minutos por pessoa; gravação de tela e áudio só com consentimento, sem rosto.

## 4. Roteiro da sessão

Abertura (5 min), aquecimento (3 min), tarefas lidas do cartão uma a uma (30 a 40 min), questionário
SUS (5 min), conversa final (5 min): o mais difícil, o que faria errado num dia corrido, o que falta.

## 5. Tarefas críticas por perfil

IDs e cenários completos no docs/26 § 9: `T-REC-01` a `T-REC-06`, `T-TEC-01` a `T-TEC-05`,
`T-MED-01` a `T-MED-06`, `T-CXA-01` a `T-CXA-05`, `T-FAT-01` a `T-FAT-04`. Cada uma tem mockup,
tempo-alvo e definição de erro crítico. Erro crítico é o que levaria a dano: paciente errado, exame
errado, valor errado, resultado no lugar errado.

Ao desenhar uma tela nova, o designer verifica se ela toca alguma tarefa crítica e, se criar tarefa
crítica nova, propõe o cenário ao produto para entrar no docs/26 (USA-01).

## 6. Métricas e critérios de aprovação

Métricas: sucesso (sem ajuda, com ajuda, não concluída), tempo, erros críticos e não críticos,
pedidos de ajuda, SUS (0 a 100).

Por tarefa crítica:
1. Pelo menos 4 de 5 participantes concluem sem erro e sem ajuda.
2. Nenhum erro crítico em nenhum participante; um erro crítico bloqueia até a tela mudar e a tarefa
   ser retestada com **outros** participantes.
3. Tempo mediano até 1,5 vez o tempo-alvo.

Por perfil: SUS médio de pelo menos 75.

## 7. Achados

Cada achado vira `ACH-USA-AAAA-NNN`, ligado a requisito (`REQ-*`, criando um se preciso) e ao teste
que comprova a correção, na matriz `docs/modelos/rastreabilidade.md`. Nenhum achado é descartado sem
motivo registrado.

| Gravidade | Critério | Tratamento |
|---|---|---|
| Crítico | Erro crítico ocorreu ou poderia ocorrer | Bloqueia o módulo; vira risco no registro de riscos clínicos se clínico; reteste obrigatório |
| Alto | Não concluída ou só com ajuda | Corrigido antes do aceite |
| Médio | Atraso ou hesitação relevante | Próxima iteração |
| Baixo | Incômodo | Lista de melhorias |

Achado de tarefa clínica vai ao responsável clínico de segurança com o pacote
`docs/modelos/pacote-responsavel-clinico.md`.

### Modelo: registro de achado

```
ACH-USA-2026-001
Rodada: R1 (mockups, commit <sha>)        Sessão: REC-P3        Data: <dd/mm/aaaa>
Perfil: Recepção                           Tarefa: T-REC-04
Tela e versão: Recepcao.dc.html (<sha>)
O que aconteceu: a participante iniciou o check-in pela primeira "Maria Helena Duarte" da lista
  sem comparar o nascimento; parou ao ver o alerta de homônimo, depois de 40 s.
Erro crítico? Não ocorreu; poderia ocorrer se o alerta passasse despercebido.
Gravidade: Crítico (poderia ocorrer)
Causa provável na tela: nascimento em cinza claro na linha da esteira; alerta só no detalhe.
Proposta: nascimento com o mesmo peso do nome nas linhas com homônimo; alerta na própria linha.
Requisito: REQ-PAC-001        Risco: RC-001        Teste de correção: <a definir com QA>
Dono: <designer>              Prazo: <data>        Situação: Aberto
```

## 8. Relatório da rodada

```
Relatório de usabilidade: Rodada <R1|R2|Final>, módulo <nome>
Versão testada: <commit ou hashes>        Período: <datas>
Participantes por perfil: Recepção 5, Técnico 5, Médico 5, Caixa 5, Faturamento 5

| Tarefa | Sem erro e sem ajuda | Erros críticos | Tempo mediano / alvo | Resultado |
|---|---|---|---|---|
| T-REC-01 | 5 de 5 | 0 | 2 min 40 s / 3 min | Aprovada |

SUS médio por perfil: <valores>
Achados: <lista ACH-USA com gravidade e dono>
Decisão: Aprovada | Reprovada (tarefas a retestar: ...)
Assinaturas: designer, dono do produto, responsável clínico (se houver tarefa clínica)
```

Gravações apagadas 90 dias após o relatório, com registro de eliminação (USA-08).
