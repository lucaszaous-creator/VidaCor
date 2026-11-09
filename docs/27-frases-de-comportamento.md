# 27 · Frases de comportamento dos fluxos críticos

> **Status:** todas **aprovadas pelo dono do produto** em 08/11/2026 (DEC-2026-11-08-49). As frases clínicas
> (CHK, PUL, TRI, LAU, CTG) aguardam o parecer do responsável clínico de segurança antes de qualquer lançamento.
> Regra G4 ([docs/15](15-padrao-de-qualidade.md)): as frases são aprovadas **antes** de
> escrever os testes. Cada frase vira pelo menos um teste de integração na rota HTTP real,
> com PostgreSQL real (Testcontainers) e Orthanc/NATS reais quando o fluxo usar; mock só
> de serviço externo (PSP do PIX, operadora, Meta, prefeitura, provedor de assinatura).
> Frases com efeito clínico (CHK, PUL, TRI, LAU, CTG) também passam pelo
> [pacote do responsável clínico](modelos/pacote-responsavel-clinico.md).

## Como aprovar

1. O dono do produto lê cada cenário e marca, na tabela da seção, **Aprovada**, **Aprovada com ajuste** (escrevendo o ajuste em Observações) ou **Recusada** (com motivo).
2. Preenche aprovador e data.
3. Frase aprovada não muda sem nova aprovação; a mudança entra como nova linha com o mesmo ID e sufixo de versão (`FC-AGD-01 v2`), e a anterior fica registrada.
4. O teste que comprova cada frase é anotado na [matriz de rastreabilidade](modelos/rastreabilidade.md).

Convenções: dados fictícios dos mockups; "a rota responde" indica o código HTTP;
mensagens ao usuário seguem o docs/16 R8 (o que aconteceu, o que foi feito, o que fazer).
Cada fluxo cobre caminho feliz, erro esperado, repetição, limite e acesso indevido.

---

## 1. Agendamento de convênio com guia

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-AGD-01 | Guia válida é conferida e consumida no agendamento | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-02 | Guia já usada por outro atendimento é recusada | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-03 | Dois agendamentos simultâneos com a mesma guia | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-04 | Clique duplo no agendar não duplica | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-05 | Procedimento fora do autorizado é recusado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-06 | Senha vencida na data do exame é recusada | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-07 | Saldo zero de procedimento é recusado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-08 | Reserva provisória sem guia expira e devolve a vaga | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-09 | Guia entregue dentro do prazo confirma a reserva | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-10 | Perfil sem permissão não agenda | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-AGD-11 | Dois agendamentos simultâneos no mesmo horário da sala | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |

```gherkin
# language: pt
Funcionalidade: Agendamento de convênio com guia

  Contexto:
    Dado o paciente "Antônio Carlos Ribeiro", nascido em 15/06/1954, com carteirinha Unimed ativa
    E a guia "UN-55120" da Unimed com senha "UN-88231-X" válida até 20/11/2026
    E a guia autoriza os procedimentos TUSS 40901114 e 40901300 com saldo 1 de cada
    E a recepcionista "Ana Souza" com o perfil Recepção

  Cenário: FC-AGD-01 Guia válida é conferida e consumida no agendamento
    Quando Ana agenda o paciente na sala 1 em 07/11/2026 às 09:00 com a guia "UN-55120" e os dois procedimentos
    Então o agendamento é criado com situação "confirmado"
    E o saldo de cada procedimento da guia passa a 0
    E a guia fica ligada a este atendimento no lastro, com quem agendou, canal e estação

  Cenário: FC-AGD-02 Guia já usada por outro atendimento é recusada
    Dado que a guia "UN-55120" já foi consumida pelo atendimento "ATD-2026-118790"
    Quando Ana tenta agendar outro atendimento com a mesma guia
    Então a rota responde 409
    E a tela informa que a guia já está em uso no atendimento "ATD-2026-118790" e que é preciso outra guia

  Cenário: FC-AGD-03 Dois agendamentos simultâneos com a mesma guia
    Quando duas recepcionistas enviam ao mesmo tempo agendamentos diferentes com a guia "UN-55120"
    Então só um agendamento é criado
    E o outro pedido recebe 409
    E o saldo da guia nunca fica negativo

  Cenário: FC-AGD-04 Clique duplo no agendar não duplica
    Quando Ana envia o mesmo agendamento duas vezes com a mesma Idempotency-Key
    Então existe um só agendamento
    E as duas respostas são iguais

  Cenário: FC-AGD-05 Procedimento fora do autorizado é recusado
    Quando Ana tenta agendar o procedimento TUSS 40901220, que não está na guia
    Então a rota responde 422
    E a tela informa qual procedimento não está autorizado e sugere pedir autorização

  Cenário: FC-AGD-06 Senha vencida na data do exame é recusada
    Quando Ana tenta agendar para 25/11/2026, depois da validade da senha
    Então a rota responde 422
    E a tela informa a validade da senha e sugere remarcar ou pedir prorrogação

  Cenário: FC-AGD-07 Saldo zero de procedimento é recusado
    Dado que o saldo do procedimento 40901114 da guia é 0
    Quando Ana tenta agendar esse procedimento com a guia
    Então a rota responde 422

  Cenário: FC-AGD-08 Reserva provisória sem guia expira e devolve a vaga
    Dado o prazo de reserva provisória configurado em 48 horas
    E Ana reservou a sala 1 em 10/11/2026 às 10:00 para um paciente de convênio sem guia
    Quando passam 48 horas sem a guia ser registrada
    Então a reserva muda para "expirada" com o motivo "guia não entregue no prazo"
    E o horário volta a aparecer como livre e é oferecido à lista de espera
    E o paciente recebe o aviso de expiração pelo canal autorizado

  Cenário: FC-AGD-09 Guia entregue dentro do prazo confirma a reserva
    Dado a mesma reserva provisória com 20 horas restantes
    Quando Ana registra uma guia válida para os procedimentos reservados
    Então a reserva vira agendamento "confirmado" e a guia é consumida

  Cenário: FC-AGD-10 Perfil sem permissão não agenda
    Dado o usuário "Marcos" com o perfil TI
    Quando Marcos tenta criar um agendamento pela rota
    Então a rota responde 403
    E nenhum agendamento nem consumo de guia é gravado

  Cenário: FC-AGD-11 Dois agendamentos simultâneos no mesmo horário da sala
    Quando duas recepcionistas agendam pacientes diferentes na sala 1 às 09:00 ao mesmo tempo
    Então só um agendamento é criado
    E o outro pedido recebe 409 com a sugestão dos próximos horários livres
```

---

## 2. Check-in com dois identificadores

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-CHK-01 | Check-in com nome e nascimento conferidos | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CHK-02 | Check-in sem os dois identificadores é bloqueado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CHK-03 | Nascimento diferente do cadastro bloqueia | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CHK-04 | Homônimo no mesmo dia gera alerta | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CHK-05 | Check-in repetido não gera duas senhas nem duas pulseiras | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CHK-06 | Check-in de agendamento cancelado é recusado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CHK-07 | Perfil Caixa não faz check-in | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |

```gherkin
# language: pt
Funcionalidade: Check-in com dois identificadores

  Contexto:
    Dado o agendamento de "Antônio Carlos Ribeiro", nascido em 15/06/1954, para hoje às 09:00 na sala 1
    E a recepcionista "Ana Souza" no guichê 1

  Cenário: FC-CHK-01 Check-in com nome e nascimento conferidos
    Quando Ana confirma o nome completo, a data de nascimento e o documento com foto
    E conclui o check-in
    Então o atendimento passa a "aguardando sala" com a senha emitida
    E a pulseira com nome, nascimento, código ETQ e ACC é impressa no guichê 1
    E o lastro registra as três conferências com quem, quando e de qual estação

  Cenário: FC-CHK-02 Check-in sem os dois identificadores é bloqueado
    Quando Ana tenta concluir o check-in sem confirmar a data de nascimento
    Então a rota responde 422
    E a tela indica que falta conferir a data de nascimento
    E nenhuma senha ou pulseira é emitida

  Cenário: FC-CHK-03 Nascimento diferente do cadastro bloqueia
    Quando o paciente informa nascimento 15/06/1955 e Ana registra a divergência
    Então o check-in fica bloqueado até a correção do cadastro com motivo ou a identificação do paciente certo

  Cenário: FC-CHK-04 Homônimo no mesmo dia gera alerta
    Dado que "Antônio Carlos Ribeiro", nascido em 02/01/1980, também está agendado hoje
    Quando Ana abre o check-in de qualquer um dos dois
    Então a tela mostra o alerta de homônimo em nível "atenção" com os dois nascimentos

  Cenário: FC-CHK-05 Check-in repetido não gera duas senhas nem duas pulseiras
    Quando o check-in é enviado duas vezes com a mesma Idempotency-Key
    Então existe uma só senha e uma só pulseira

  Cenário: FC-CHK-06 Check-in de agendamento cancelado é recusado
    Dado que o agendamento foi cancelado com motivo
    Quando Ana tenta fazer o check-in
    Então a rota responde 409

  Cenário: FC-CHK-07 Perfil Caixa não faz check-in
    Dado o usuário "Carla" com o perfil Caixa apenas
    Quando Carla tenta concluir o check-in pela rota
    Então a rota responde 403
```

---

## 3. Bipe da pulseira na sala

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-PUL-01 | Pulseira certa libera a worklist | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-PUL-02 | Pulseira de outro paciente bloqueia o exame | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-PUL-03 | Pulseira invalidada bloqueia | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-PUL-04 | Bipe repetido não envia a worklist duas vezes | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-PUL-05 | Técnico de outra sala não libera o exame | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |

```gherkin
# language: pt
Funcionalidade: Bipe da pulseira na sala

  Contexto:
    Dado o técnico "Bruno" na sala 1 com o aparelho "Vivid E95" cadastrado
    E o exame de "Antônio Carlos Ribeiro", 15/06/1954, agendado para a sala 1, pulseira "ETQ-2026-018742"

  Cenário: FC-PUL-01 Pulseira certa libera a worklist
    Quando Bruno bipa a pulseira "ETQ-2026-018742"
    Então o item da worklist é enviado ao "Vivid E95"
    E o lastro registra o bipe com sala, aparelho, técnico e hora

  Cenário: FC-PUL-02 Pulseira de outro paciente bloqueia o exame
    Dado a pulseira "ETQ-2026-018755" de "Juliana Sampaio Rocha", 12/04/1993, agendada para a sala 2
    Quando Bruno bipa essa pulseira na sala 1
    Então o exame fica bloqueado na sala 1
    E nenhum item de worklist é enviado ao aparelho
    E uma divergência é aberta com as duas identificações
    E a tela informa que a paciente é da sala 2 e oferece encaminhá-la

  Cenário: FC-PUL-03 Pulseira invalidada bloqueia
    Dado que a pulseira "ETQ-2026-018742" foi reimpressa e invalidada
    Quando Bruno bipa a pulseira antiga
    Então o ato é bloqueado e a tela informa que a etiqueta foi substituída

  Cenário: FC-PUL-04 Bipe repetido não envia a worklist duas vezes
    Quando Bruno bipa a mesma pulseira duas vezes seguidas
    Então existe um só item de worklist para o exame

  Cenário: FC-PUL-05 Técnico de outra sala não libera o exame
    Dado o técnico "Ricardo" lotado na sala 6
    Quando Ricardo tenta liberar pela rota o exame da sala 1
    Então a rota responde 403
```

---

## 4. Triagem com contraste bloqueante

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-TRI-01 | Triagem sem risco libera o exame | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-TRI-02 | Reação prévia ou TFG baixa exige aprovação médica | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-TRI-03 | Exame bloqueado não recebe worklist | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-TRI-04 | Médico registra a conduta e libera | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-TRI-05 | Técnico não aprova a própria triagem | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-TRI-06 | Exame com contraste sem triagem não inicia | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-TRI-07 | TFG é calculada pelo servidor | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |

```gherkin
# language: pt
Funcionalidade: Triagem de segurança com contraste

  Contexto:
    Dado o exame "TC de abdome com contraste" de "Vera Lúcia Antunes", 18/09/1957, na sala 6
    E a enfermeira "Lívia Costa" com o perfil Técnico de sala na sala 6
    E a médica "Dra. Cecília Souto" com vínculo de cuidado com a paciente

  Cenário: FC-TRI-01 Triagem sem risco libera o exame
    Quando Lívia registra a triagem sem reação prévia, TFG acima do limite e sem gestação
    Então o resultado é "liberado"
    E o exame pode ser iniciado

  Cenário: FC-TRI-02 Reação prévia ou TFG baixa exige aprovação médica
    Quando Lívia registra creatinina 1,4 mg/dL de 28/10/2026
    E o servidor calcula a TFG de 40 mL/min/1,73 m² pelo CKD-EPI
    Então o resultado é "precisa de aprovação médica"
    E a aprovação é pedida à Dra. Cecília Souto

  Cenário: FC-TRI-03 Exame bloqueado não recebe worklist
    Dado a triagem com resultado "precisa de aprovação médica" sem conduta registrada
    Quando a pulseira certa é bipada na sala 6
    Então a worklist não é enviada e a tela informa que falta a conduta médica

  Cenário: FC-TRI-04 Médico registra a conduta e libera
    Quando a Dra. Cecília registra a conduta "prosseguir com hidratação" com reautenticação
    Então o exame é liberado
    E o lastro grava médica, conduta, horário e versão da regra de triagem

  Cenário: FC-TRI-05 Técnico não aprova a própria triagem
    Quando Lívia tenta registrar a conduta médica pela rota
    Então a rota responde 403

  Cenário: FC-TRI-06 Exame com contraste sem triagem não inicia
    Dado que não há triagem registrada para o exame
    Quando o técnico tenta iniciar o exame
    Então a rota responde 422

  Cenário: FC-TRI-07 TFG é calculada pelo servidor
    Quando a tela envia uma TFG de 90 junto com a creatinina 1,4 mg/dL
    Então o servidor ignora o valor enviado e grava a TFG calculada de 40
```

---

## 5. Laudo assinado, portal e correção

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-LAU-01 | Assinar laudo com confirmação nominal e reautenticação | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-02 | Laudo em rascunho não vai ao portal | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-03 | Laudo assinado vai ao portal e o paciente é avisado uma vez | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-04 | Laudo assinado não pode ser alterado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-05 | Correção cria versão 2 com motivo | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-06 | Correção sem motivo é recusada | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-07 | Duas assinaturas simultâneas geram uma versão vigente | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-08 | Marcação "conversar antes" segura a publicação | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-09 | Estudo sem conciliação não entra na fila de laudo | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-LAU-10 | Médico sem atribuição não assina | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |

```gherkin
# language: pt
Funcionalidade: Laudo assinado, publicação no portal e correção

  Contexto:
    Dado o laudo em rascunho do ecocardiograma de "Antônio Carlos Ribeiro", 15/06/1954
    E o laudo atribuído ao "Dr. Paulo Medeiros", com certificado ICP-Brasil válido

  Cenário: FC-LAU-01 Assinar laudo com confirmação nominal e reautenticação
    Quando o Dr. Paulo pede para assinar
    Então a confirmação mostra "Assinar o laudo de Antônio Carlos Ribeiro (15/06/1954)?"
    E exige o PIN do certificado
    Quando ele confirma
    Então a versão 1 é gravada como assinada, com certificado, carimbo do tempo e hash do PDF

  Cenário: FC-LAU-02 Laudo em rascunho não vai ao portal
    Quando qualquer rotina tenta publicar o rascunho no portal
    Então nada é publicado e a rota responde 409

  Cenário: FC-LAU-03 Laudo assinado vai ao portal e o paciente é avisado uma vez
    Dado o laudo assinado sem marcação "conversar antes"
    Quando o evento de laudo assinado é processado duas vezes
    Então o laudo aparece uma vez no portal do paciente
    E um só aviso é enviado pelo canal autorizado

  Cenário: FC-LAU-04 Laudo assinado não pode ser alterado
    Dado a versão 1 assinada
    Quando alguém tenta alterar o texto da versão 1
    Então a rota responde 409 e o texto da versão 1 continua igual

  Cenário: FC-LAU-05 Correção cria versão 2 com motivo
    Quando o Dr. Paulo corrige o laudo com o motivo "medida do átrio esquerdo digitada errada"
    E assina a correção
    Então a versão 2 passa a vigente, a versão 1 continua consultável com a marca "substituída"
    E o portal mostra a versão 2 com a indicação de retificação
    E o lastro grava o que mudou, quem, quando e o motivo

  Cenário: FC-LAU-06 Correção sem motivo é recusada
    Quando o Dr. Paulo tenta salvar a correção sem motivo
    Então a rota responde 422

  Cenário: FC-LAU-07 Duas assinaturas simultâneas geram uma versão vigente
    Quando duas requisições de assinatura do mesmo laudo chegam ao mesmo tempo
    Então só uma versão fica vigente e a outra requisição recebe 409

  Cenário: FC-LAU-08 Marcação "conversar antes" segura a publicação
    Dado o laudo marcado "conversar antes de liberar"
    Quando ele é assinado
    Então não é publicado no portal até o médico liberar depois da conversa
    E a liberação grava quem liberou e quando

  Cenário: FC-LAU-09 Estudo sem conciliação não entra na fila de laudo
    Dado um estudo recebido do aparelho sem paciente conciliado
    Quando a fila de laudos é consultada
    Então o estudo não aparece na fila de nenhum médico

  Cenário: FC-LAU-10 Médico sem atribuição não assina
    Dado o "Dr. Marcos Freire" sem atribuição neste laudo
    Quando ele tenta assinar pela rota
    Então a rota responde 403
```

---

## 6. Caixa do particular

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-CXA-01 | Total calculado pelo servidor | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-02 | Total enviado pela tela é ignorado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-03 | Desconto dentro do limite do perfil | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-04 | Desconto acima do limite sem autorização é recusado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-05 | Gerente autoriza desconto com reautenticação | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-06 | Recebimento repetido não duplica | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-07 | Valor zero ou negativo é recusado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-08 | Recebimento sem caixa aberto é recusado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-CXA-09 | Quem pede o estorno não aprova | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |

```gherkin
# language: pt
Funcionalidade: Caixa do particular

  Contexto:
    Dado a tabela de preços do particular versão 12, vigente hoje
    E o ecocardiograma custa R$ 380,00 nessa tabela
    E a operadora de caixa "Carla" com caixa aberto e limite de desconto de 5%
    E a gerente "Patrícia Lemos" com permissão de autorizar desconto

  Cenário: FC-CXA-01 Total calculado pelo servidor
    Quando Carla registra o ecocardiograma de "Sônia Maria Castro"
    Então o total é R$ 380,00 e o recebimento grava a versão 12 da tabela

  Cenário: FC-CXA-02 Total enviado pela tela é ignorado
    Quando a tela envia o total de R$ 100,00 para o mesmo item
    Então o servidor grava R$ 380,00

  Cenário: FC-CXA-03 Desconto dentro do limite do perfil
    Quando Carla aplica desconto de 5% com motivo
    Então o total é R$ 361,00 e o desconto grava quem aplicou e o motivo

  Cenário: FC-CXA-04 Desconto acima do limite sem autorização é recusado
    Quando Carla aplica desconto de 15% sem autorização
    Então a rota responde 403 e a tela oferece pedir autorização à gerência

  Cenário: FC-CXA-05 Gerente autoriza desconto com reautenticação
    Quando Patrícia autoriza o desconto de 15% com segundo fator
    Então o total é R$ 323,00 e o lastro grava quem pediu, quem autorizou e o motivo

  Cenário: FC-CXA-06 Recebimento repetido não duplica
    Quando o mesmo recebimento é enviado duas vezes com a mesma Idempotency-Key
    Então existe um só recebimento e uma só cobrança PIX

  Cenário: FC-CXA-07 Valor zero ou negativo é recusado
    Quando a tela envia um item com valor zero ou negativo
    Então a rota responde 422

  Cenário: FC-CXA-08 Recebimento sem caixa aberto é recusado
    Dado que Carla fechou o caixa
    Quando tenta registrar um recebimento
    Então a rota responde 409 e a tela orienta abrir o caixa

  Cenário: FC-CXA-09 Quem pede o estorno não aprova
    Dado que Carla pediu o estorno de um recebimento
    Quando Carla tenta aprovar o próprio pedido
    Então a rota responde 403
```

---

## 7. Webhook PIX duplicado

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-PIX-01 | Pagamento confirmado pelo PSP baixa o recebimento | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-PIX-02 | Webhook duplicado não gera segundo efeito | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-PIX-03 | Assinatura inválida é recusada antes de processar | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-PIX-04 | Eventos fora de ordem não regridem a situação | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-PIX-05 | Dois webhooks iguais ao mesmo tempo | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |

```gherkin
# language: pt
Funcionalidade: Webhook do PIX

  Contexto:
    Dado a cobrança PIX de R$ 380,00 do recebimento "REC-2026-004410" com txid gerado uma vez

  Cenário: FC-PIX-01 Pagamento confirmado pelo PSP baixa o recebimento
    Quando chega o webhook do PSP com assinatura válida informando o pagamento do txid
    Então o ID do evento é gravado antes de qualquer ação
    E o servidor responde 2xx
    E o recebimento passa a "pago" com o endToEndId
    E a emissão da NFS-e é pedida uma vez

  Cenário: FC-PIX-02 Webhook duplicado não gera segundo efeito
    Dado que o evento já foi processado
    Quando o mesmo webhook chega de novo
    Então o servidor responde 200 e para
    E existe uma só baixa e um só pedido de NFS-e

  Cenário: FC-PIX-03 Assinatura inválida é recusada antes de processar
    Quando chega um webhook com assinatura que não confere com o corpo bruto
    Então o servidor responde 401
    E nenhum evento é gravado e o recebimento não muda

  Cenário: FC-PIX-04 Eventos fora de ordem não regridem a situação
    Dado que o recebimento já está "pago"
    Quando chega depois um evento antigo de "cobrança criada"
    Então o servidor consulta o estado atual no PSP
    E o recebimento continua "pago"

  Cenário: FC-PIX-05 Dois webhooks iguais ao mesmo tempo
    Quando dois webhooks com o mesmo ID de evento chegam ao mesmo tempo
    Então só um é processado e o recebimento é baixado uma vez
```

---

## 8. Etiqueta reimpressa invalida a anterior

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-ETQ-01 | Reimpressão com motivo invalida a anterior | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ETQ-02 | Reimpressão sem motivo é recusada | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ETQ-03 | Impressão duplicada gera uma etiqueta | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ETQ-04 | Correção de nascimento invalida as etiquetas | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ETQ-05 | Perfil sem permissão não reimprime | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |

```gherkin
# language: pt
Funcionalidade: Reimpressão de etiqueta

  Contexto:
    Dado a pulseira "ETQ-2026-018742" de "Antônio Carlos Ribeiro", 15/06/1954, válida

  Cenário: FC-ETQ-01 Reimpressão com motivo invalida a anterior
    Quando a recepção reimprime a pulseira com o motivo "rasgou"
    Então uma nova etiqueta com código novo é criada
    E a "ETQ-2026-018742" passa a "invalidada" com o motivo e quem reimprimiu
    E bipar a etiqueta antiga bloqueia o ato

  Cenário: FC-ETQ-02 Reimpressão sem motivo é recusada
    Quando a recepção tenta reimprimir sem motivo
    Então a rota responde 422

  Cenário: FC-ETQ-03 Impressão duplicada gera uma etiqueta
    Quando o pedido de impressão é enviado duas vezes ao mesmo tempo com a mesma Idempotency-Key
    Então existe uma só etiqueta nova

  Cenário: FC-ETQ-04 Correção de nascimento invalida as etiquetas
    Quando o cadastro é corrigido de 15/06/1954 para 15/06/1955 com motivo
    Então todas as etiquetas válidas do paciente são invalidadas
    E a tela pede a reimpressão

  Cenário: FC-ETQ-05 Perfil sem permissão não reimprime
    Dado o usuário com o perfil Financeiro
    Quando tenta reimprimir a pulseira pela rota
    Então a rota responde 403
```

---

## 9. Acesso indevido

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-ACE-01 | Paciente A não vê laudo de B pelo portal | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ACE-02 | Paciente A não descobre se o ID de B existe | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ACE-03 | Médico sem vínculo de cuidado não lê laudo | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ACE-04 | Acesso de emergência com justificativa é auditado | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ACE-05 | RLS barra mesmo se a rota errar | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ACE-06 | Recepção não lê laudo | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ACE-07 | TI não lê dado clínico | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |
| FC-ACE-08 | Link compartilhado expirado não abre | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto |

```gherkin
# language: pt
Funcionalidade: Acesso indevido

  Contexto:
    Dado a paciente A "Sônia Maria Castro" e o paciente B "Antônio Carlos Ribeiro", ambos com laudos assinados e liberados

  Cenário: FC-ACE-01 Paciente A não vê laudo de B pelo portal
    Quando a paciente A, logada no portal, pede o laudo de B pelo ID
    Então a rota responde 404
    E a tentativa é registrada na auditoria

  Cenário: FC-ACE-02 Paciente A não descobre se o ID de B existe
    Quando a paciente A pede um ID que existe e um ID que não existe
    Então as duas respostas são iguais, com o mesmo código e o mesmo formato

  Cenário: FC-ACE-03 Médico sem vínculo de cuidado não lê laudo
    Dado o "Dr. Marcos Freire" sem agendamento, exame, laudo atribuído ou atendimento aberto com B
    Quando ele pede o laudo de B
    Então a rota responde 403
    E a tela oferece o acesso de emergência com justificativa

  Cenário: FC-ACE-04 Acesso de emergência com justificativa é auditado
    Quando o Dr. Marcos usa o acesso de emergência com a justificativa "paciente em atendimento de urgência no pronto-socorro"
    Então ele lê o laudo
    E o acesso é registrado em destaque, avisado ao responsável clínico de segurança e entra na fila de revisão de 48 horas

  Cenário: FC-ACE-05 RLS barra mesmo se a rota errar
    Dado uma transação com o contexto do Dr. Marcos sem vínculo e sem acesso de emergência
    Quando uma consulta direta ao banco pede os laudos de B
    Então o banco não devolve nenhuma linha

  Cenário: FC-ACE-06 Recepção não lê laudo
    Dado a recepcionista "Ana Souza"
    Quando ela pede o conteúdo do laudo de B
    Então a rota responde 403
    E ela continua vendo apenas a situação do laudo, sem o conteúdo

  Cenário: FC-ACE-07 TI não lê dado clínico
    Dado o usuário "Marcos" com o perfil TI
    Quando ele pede qualquer laudo, evolução ou imagem
    Então a rota responde 403

  Cenário: FC-ACE-08 Link compartilhado expirado não abre
    Dado o link do laudo de A compartilhado com médico externo e expirado ontem
    Quando o link é aberto
    Então nada é exibido e a tela informa que o link expirou
```

---

## 10. Contingência e lançamento posterior

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-CTG-01 | Declaração de contingência | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-02 | Declaração feita offline sobe uma vez | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-03 | Ficha lançada leva a marca de contingência | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-04 | Mesma ficha lançada duas vezes gera um registro | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-05 | Ficha com número fora da faixa é recusada | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-06 | Nascimento diferente no papel abre divergência | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-07 | Valor do papel diferente do recalculado abre divergência | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-08 | Estudo de contingência não vai direto ao laudo | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-09 | Encerramento bloqueado com pendência | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-10 | Perfil sem permissão não declara nem encerra | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |
| FC-CTG-11 | Kit das 06:00 é gerado com faixa reservada | Aprovada | Lucas (dono do produto) | 08/11/2026 | Aprovada pelo dono do produto; aguarda parecer do responsável clínico de segurança antes do lançamento |

```gherkin
# language: pt
Funcionalidade: Contingência e lançamento posterior

  Contexto:
    Dado a contingência "CTG-2026-0007" declarada às 10:42 por "Ana Souza", responsável "Patrícia Lemos"
    E a faixa de fichas F-ATD 0400 a 0499 reservada para hoje

  Cenário: FC-CTG-11 Kit das 06:00 é gerado com faixa reservada
    Quando a tarefa das 06:00 roda
    Então o PDF da agenda por sala e as fichas são gerados com versão do modelo e hash no lastro
    E a faixa de numeração do dia é reservada sem sobrepor faixas anteriores

  Cenário: FC-CTG-01 Declaração de contingência
    Quando Ana declara contingência total com o motivo "servidor local fora do ar"
    Então a contingência fica aberta com nível, motivo, quem declarou, responsável e hora
    E todas as estações e a coordenação são avisadas

  Cenário: FC-CTG-02 Declaração feita offline sobe uma vez
    Dado a declaração gravada na estação do guichê 1 sem servidor
    Quando o servidor volta e a estação envia a declaração três vezes
    Então existe uma só contingência com a hora original da declaração

  Cenário: FC-CTG-03 Ficha lançada leva a marca de contingência
    Quando a recepção lança a ficha F-ATD 0412 de "Maria Helena Duarte", 12/03/1968, atendida às 10:50 pelo Dr. Paulo Medeiros
    Então o atendimento é criado com origem "contingência", contingência "CTG-2026-0007", formulário "F-ATD 0412"
    E grava ocorrido às 10:50, quem atendeu, quem lançou e a hora do lançamento
    E o PDF digitalizado da ficha fica anexado com hash
    E a tela e o explorador de lastro mostram "Registrado em contingência"

  Cenário: FC-CTG-04 Mesma ficha lançada duas vezes gera um registro
    Quando duas estações lançam a ficha F-ATD 0412 ao mesmo tempo
    Então existe um só atendimento
    E a segunda estação recebe 409 com o atendimento já lançado

  Cenário: FC-CTG-05 Ficha com número fora da faixa é recusada
    Quando alguém lança a ficha F-ATD 0612, que não foi reservada
    Então a rota responde 422

  Cenário: FC-CTG-06 Nascimento diferente no papel abre divergência
    Dado o cadastro de "Juliana Sampaio Rocha" com nascimento 12/04/1993
    Quando a ficha F-ATD 0414 é lançada com nascimento 12/04/1992
    Então nenhum cadastro é alterado nem unido
    E uma divergência de identificação é aberta com dono e prazo

  Cenário: FC-CTG-07 Valor do papel diferente do recalculado abre divergência
    Quando o recibo manual F-REC 0207 é lançado com R$ 300,00 para um exame de R$ 380,00 na tabela vigente na data
    Então o servidor grava o valor recalculado e abre divergência de preço

  Cenário: FC-CTG-08 Estudo de contingência não vai direto ao laudo
    Quando o aparelho reenvia um estudo com Patient ID "C-0414"
    Então o estudo vai para a fila de conciliação de contingência
    E só entra na fila de laudo depois da conciliação com nome e nascimento conferidos

  Cenário: FC-CTG-09 Encerramento bloqueado com pendência
    Dado uma ficha pendente, uma divergência de identificação aberta ou um número da faixa sem situação
    Quando Patrícia tenta encerrar a contingência
    Então a rota responde 409 e a tela lista o que falta

  Cenário: FC-CTG-10 Perfil sem permissão não declara nem encerra
    Dado o usuário "Carla" com o perfil Caixa
    Quando Carla tenta declarar ou encerrar a contingência pela rota
    Então a rota responde 403
```

## 31. Minha agenda do médico e ausência (proposta, aguardando aprovação)

> Escritas pelo Agente 2 em 09/11/2026 (DEC-2026-11-09-140 a 143). Testadas em `crates/api/tests/minha_agenda.rs`.
> Valem como proposta até o dono do produto aprovar; mudança depois da aprovação segue a regra de versão acima.

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-MAG-01 | O médico vê só os próprios pacientes, com nome e nascimento | Proposta, aguardando aprovação | — | — | Base: App Agenda da Medware (agenda do médico) |
| FC-MAG-02 | Depois do check-in o paciente aparece como esperando | Proposta, aguardando aprovação | — | — | — |
| FC-MAG-03 | Quem não é médico não abre a agenda do médico | Proposta, aguardando aprovação | — | — | — |
| FC-MAG-04 | O bloqueio pedido só fecha a agenda depois da aprovação | Proposta, aguardando aprovação | — | — | Segunda pessoa (docs/18) |
| FC-MAG-05 | O médico não aprova o próprio pedido; recusa exige motivo | Proposta, aguardando aprovação | — | — | — |
| FC-MAG-06 | Outro médico não vê nem cancela o pedido | Proposta, aguardando aprovação | — | — | — |
| FC-MAG-07 | Período inválido é recusado | Proposta, aguardando aprovação | — | — | Até 62 dias por pedido |
| FC-MAG-08 | Clique duplo e pedidos simultâneos não duplicam | Proposta, aguardando aprovação | — | — | G2 e G3 |

```gherkin
Funcionalidade: Minha agenda do médico

  Cenário: FC-MAG-01 O médico vê só os próprios pacientes
    Dado "Antônio Carlos Ribeiro" (15/06/1954) agendado hoje com o Dr. Paulo Medeiros
    E "Vera Lúcia Antunes" agendada com a Dra. Cecília Souto
    Quando o Dr. Paulo abre "Minha agenda"
    Então ele vê Antônio, com nome e nascimento, no grupo "A chegar"
    E não vê Vera

  Cenário: FC-MAG-02 Check-in leva o paciente a "Esperando"
    Quando a recepção faz o check-in de Antônio
    Então na agenda do Dr. Paulo Antônio aparece em "Esperando", com a senha e desde quando espera

  Cenário: FC-MAG-03 Só o médico abre a própria agenda
    Quando recepção, caixa, técnico, gerência, TI ou faturamento pedem "Minha agenda"
    Então a rota responde 403

  Cenário: FC-MAG-04 Bloqueio só vale depois da aprovação
    Dado um paciente já agendado com o Dr. Paulo daqui a 3 dias
    Quando o Dr. Paulo pede o bloqueio de 3 a 5 dias à frente por congresso
    Então o pedido fica "aguardando" com 1 paciente afetado e a agenda continua aberta
    Quando Patrícia (coordenação) aprova
    Então um agendamento novo com o Dr. Paulo no período é recusado com 409 "medico-ausente"
    E com outro médico no mesmo período o agendamento entra

  Cenário: FC-MAG-05 Segunda pessoa decide
    Quando o próprio Dr. Paulo ou a recepção tentam aprovar o pedido
    Então a rota responde 403
    E a recusa sem motivo responde 422, e um pedido já decidido responde 409

  Cenário: FC-MAG-06 Acesso indevido
    Quando a Dra. Cecília lista ou tenta cancelar o pedido do Dr. Paulo
    Então ela não o vê e o cancelamento responde 404

  Cenário: FC-MAG-07 Limites
    Quando o período é vazio, passa de 62 dias, já passou ou o motivo não é da lista
    Então a rota responde 422 e nada é gravado

  Cenário: FC-MAG-08 Repetição e concorrência
    Quando o mesmo pedido é enviado duas vezes com a mesma chave
    Então volta a mesma resposta e há um registro só
    Quando dois pedidos sobrepostos chegam ao mesmo tempo
    Então só um entra e o outro recebe 409
    Quando duas decisões chegam ao mesmo tempo sobre o mesmo pedido
    Então só uma tem efeito
```

## 32. Gestão de dose (proposta, aguardando aprovação)

> Escritas pelo Agente 2 em 09/11/2026 (DEC-2026-11-09-144 a 149). Testadas em `crates/api/tests/gestao_dose.rs`
> com Orthanc real recebendo imagem e RDSR. Frases clínicas: precisam também do parecer do responsável clínico de segurança.

| ID | Cenário | Status | Aprovador | Data | Observações |
|---|---|---|---|---|---|
| FC-DOS-01 | RDSR do aparelho vira dose comparada ao NRD | Proposta, aguardando aprovação | — | — | RDC 611/2022 |
| FC-DOS-02 | Sem vínculo o médico não vê a dose; a gerência só vê o agregado | Proposta, aguardando aprovação | — | — | docs/18 N3 |
| FC-DOS-03 | Justificativa uma vez só, por quem tem vínculo | Proposta, aguardando aprovação | — | — | — |
| FC-DOS-04 | Dose dentro do NRD não pede justificativa | Proposta, aguardando aprovação | — | — | — |
| FC-DOS-05 | NRD só a gerência publica, com grandeza certa e versão | Proposta, aguardando aprovação | — | — | — |
| FC-DOS-06 | Duas publicações simultâneas do NRD: só uma entra | Proposta, aguardando aprovação | — | — | G2 |
| FC-DOS-07 | Período inválido é recusado | Proposta, aguardando aprovação | — | — | Até 92 dias |

```gherkin
Funcionalidade: Gestão de dose

  Cenário: FC-DOS-01 RDSR vira dose comparada ao NRD
    Dado o NRD da TC de abdome com contraste em CTDIvol 12 mGy, com fator 0,015 mSv/mGy·cm
    E "Vera Lúcia Antunes" (18/09/1957) com check-in para a TC com a Dra. Cecília Souto
    Quando o tomógrafo manda a imagem e o RDSR com CTDIvol 14,2 mGy e DLP 520 mGy·cm
    Então o núcleo concilia o estudo, grava a dose com o objeto RDSR e o SHA-256 das tags lidas
    E marca "acima do NRD"
    E a Dra. Cecília vê a dose com nome e nascimento e 7,8 mSv estimados em 12 meses
    E ler de novo não grava outra dose

  Cenário: FC-DOS-02 Acesso indevido
    Quando o Dr. Paulo, sem vínculo com Vera, abre a gestão de dose
    Então ele não vê o exame e o histórico de Vera responde 404
    E Patrícia (gerência) recebe 403 na lista por paciente e vê só o agregado por protocolo, sem paciente
    E recepção, técnico, caixa e TI recebem 403 até no agregado

  Cenário: FC-DOS-03 Justificativa
    Quando o Dr. Paulo tenta justificar, a rota responde 404
    E "Outro" sem descrição responde 422
    Quando a Dra. Cecília justifica "fase extra" duas vezes com a mesma chave
    Então volta a mesma resposta; com outra chave, 409 "dose-ja-justificada"

  Cenário: FC-DOS-04 Dose dentro do NRD
    Dado NRD 15 mGy e dose de 9,8 mGy
    Quando alguém tenta justificar
    Então a rota responde 409 "dose-dentro-do-nrd"

  Cenário: FC-DOS-05 Publicação do NRD
    Quando um médico publica o NRD, a rota responde 403
    E na TC em DAP, com valor zero, negativo ou acima de 100.000, responde 422
    E a segunda versão exige a versão vista e o motivo; a anterior fica guardada

  Cenário: FC-DOS-06 Concorrência
    Quando duas publicações do primeiro NRD do mesmo exame chegam juntas
    Então só uma entra e a outra recebe 409

  Cenário: FC-DOS-07 Limites
    Quando o período passa de 92 dias ou o início é depois do fim
    Então a rota responde 422
```
