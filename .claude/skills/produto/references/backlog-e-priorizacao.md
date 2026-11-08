# Backlog e priorização

Fontes: `docs/25-plano-do-projeto.md` §2, §3 e §5; `docs/22-lacunas-e-roadmap.md`;
`docs/04-escopo-funcional.md`; `docs/11-arquitetura-e-infraestrutura.md` §5.

## 1. O que define o MVP

O MVP é o conjunto mínimo que permite **desligar a Medware e a Medilab** sem perder função
usada hoje, mais os essenciais E1 a E8 do `docs/22` (DEC-2026-11-08-26). A tabela completa,
módulo a módulo, está no `docs/25` §2. Resumo para triagem rápida:

| Classe | Itens | Onde entra |
|---|---|---|
| MVP | Identidade, pacientes, agenda, agendamento com guia, autorização (E4), recepção, etiquetas, caixa com PIX e NFS-e, lastro, contingência (E6), sala e aparelhos, triagem (E1), laudo e laudo estruturado (E2), exame externo (E3), visualizador, fluxo de laudos, portal, faturamento e auditoria (E5), financeiro essencial, estoque com lote, WhatsApp básico, indicadores básicos, configurações, migração (E7) | Fases 0 a 4 |
| Depois do MVP | Estacionamento, apps, Meus Exames Bebê, telemedicina, I1 a I9, F1 a F6 | Fase 5 |
| Fora | CD/DVD (retirado em 07/11/2026); comunicação obrigatória de achado crítico (descartado, pode voltar se a clínica pedir) | Não entra |

Pergunta de corte para qualquer pedido novo: **"sem isto, a clínica consegue desligar a
Medware e a Medilab sem perder o que usa hoje?"** Se consegue, o item vai para depois do
MVP, salvo decisão do comitê registrada.

## 2. Fases e marcos

| Fase | Meses (estimativa) | Marco | Conteúdo resumido |
|---|---|---|---|
| 0 Descoberta e fundação | 1 a 2 | M0 | Inventário, contrato, ADRs, frases aprovadas, esqueleto do monorepo, homologação, usabilidade R1 com mockups, pedido de exportação |
| 1 Atendimento administrativo | 3 a 6 | M1 | Identidade, pacientes, agenda, guia, autorização, recepção, etiquetas, caixa, lastro, contingência, convite do portal |
| 2 Clínico e imagem | 6 a 10 | M2 | Orthanc, worklist, MPPS, gateway, sala, triagem, laudo, assinatura, visualizador, externo, portal com laudos, WhatsApp básico |
| 3 Faturamento e gestão | 9 a 13 | M3 | TISS, auditoria, financeiro, estoque, indicadores |
| 4 Migração e virada | 12 a 16 | M4 e M5 | Ondas do `docs/24`, piloto por sala, virada, fim da janela de volta |
| 5 Depois do MVP | 17 em diante | Por item | I1 a I9, apps, estacionamento, telemedicina, F1 a F6 |

As fases se sobrepõem de propósito. Datas de calendário só depois de confirmar as premissas
(PREM-05 e as demais do `docs/25` §1).

## 3. Épicos por módulo

Um épico por módulo do núcleo (`docs/11` §5), mais os transversais. Nome do épico = nome do
módulo, para casar com crate, schema e prefixo de requisito.

| Épico | Módulo | Fase principal |
|---|---|---|
| Identidade e perfis | `identidade` | 1 |
| Auditoria e lastro | `auditoria` | 1 |
| Pacientes | `pacientes` | 1 |
| Agenda e agendamento com guia | `agenda` | 1 |
| Recepção, senhas e etiquetas | `recepcao` | 1 |
| Caixa | `caixa` | 1 |
| Contingência | transversal | 1 |
| Exames, sala e aparelhos | `exames` | 2 |
| Laudos | `laudos` | 2 |
| Atendimento médico | `atendimento` | 2 (se a clínica faz consulta hoje) |
| Portal do paciente | portal | 1 e 2 |
| Comunicação (WhatsApp e NPS) | `comunicacao` | 2 |
| Faturamento TISS e auditoria | `faturamento` | 3 |
| Financeiro | `financeiro` | 3 |
| Estoque | `estoque` | 3 |
| Indicadores | `indicadores` | 3 |
| Migração | transversal | 4 |
| Estacionamento | `estacionamento` | 5 |

## 4. Método de priorização (a confirmar)

O `docs/25` define o que é MVP e a ordem das fases, mas **não fixa um método de pontuação**
dentro de cada fase. Proposta pela regra do ADR-0019, a confirmar pelo dono do produto e a
registrar em `docs/decisoes.md` quando aprovada:

1. **Bloqueadores primeiro:** dependência técnica de outros itens (identidade antes de tudo, agenda antes de check-in, worklist antes de laudo).
2. **Pontuação** de 1 a 5 em quatro critérios:

| Critério | Pergunta | Peso proposto |
|---|---|---|
| Desligamento | Quanto aproxima o desligamento da Medware/Medilab? | 3 |
| Risco clínico | Reduz ou controla um risco `RC-*` ou `RCP-*`? | 3 |
| Risco do projeto | Retira incerteza cedo (integração nova, aparelho, operadora)? | 2 |
| Esforço | Quanto menor, maior a nota | 1 |

3. Empate: o item com **frase de comportamento já aprovada** vai antes.
4. Risco clínico alto sem controle definido **não entra** na iteração, qualquer que seja a nota.

Até a confirmação, o assistente usa esta proposta e diz, em cada priorização, que o método
ainda está "a confirmar".

## 5. Fatiamento vertical

Cada item do backlog entrega valor testável de ponta a ponta: tela, rota, regra no núcleo,
banco, lastro e teste. Proibido fatiar por camada ("criar tabelas da agenda", "fazer a tela
da agenda").

Exemplo de épico "Agenda e agendamento com guia" fatiado:

| Fatia | Valor entregue | Frases ligadas |
|---|---|---|
| 1 | Recepção vê a agenda do dia por sala com horários livres clicáveis | (a escrever) |
| 2 | Agendar particular em horário livre, sem duplicar em clique duplo | FC-AGD-04, FC-AGD-11 |
| 3 | Agendar convênio com guia válida, consumida de forma atômica | FC-AGD-01 a FC-AGD-03 |
| 4 | Recusas da guia: procedimento fora, senha vencida, saldo zero | FC-AGD-05 a FC-AGD-07 |
| 5 | Reserva provisória sem guia com prazo e confirmação | FC-AGD-08, FC-AGD-09 |
| 6 | Perfil sem permissão não agenda | FC-AGD-10 |

A primeira entrega do projeto é a rota de ponta a ponta do `docs/11` §11: login, agenda do
dia, check-in, worklist no Orthanc e imagem de exemplo chegando na tela por WebSocket.

## 6. Regras do backlog

- Cada item aponta para épico, fase, marco, requisitos (`REQ-*`) e frases (`FC-*`).
- Bug com teste falhando entra com o ID de `docs/bugs.md` e prioridade pela gravidade (S1 a S4 do `docs/16` R14).
- Achado de usabilidade crítico ou alto (`ACH-USA-*`) entra antes do aceite do módulo (`docs/26` §12).
- Débito técnico entra como item com motivo e dono; não fica escondido em outra história.
- Item pedido por conversa sem pedido formal fica em "Triagem" até virar pedido de mudança de escopo.
- Orçamento de erro estourado (`docs/16` C6.1) congela funcionalidade nova: só correções até normalizar.

## 7. Modelos

```markdown
## Épico: <nome do módulo>
- Módulo: `<modulo>` | Fase: <0-5> | Marco: <M0-M5>
- Problema: <o que a clínica sofre hoje, em uma frase>
- Resultado esperado: <como a clínica saberá que deu certo, com métrica do docs/12 §4 quando houver>
- Fora do escopo: <o que não entra>
- Riscos clínicos ligados: <RC-xxx ou "não se aplica" com motivo>
- Fatias:
  1. <fatia> | REQ: <ids> | FC: <ids> | Situação: <Planejada/Em andamento/Aceita>
- Dependências: <épicos, integrações, premissas PREM-xx>
- Dono técnico: <nome>
```

```markdown
## Priorização da iteração <AAAA-NN> (método a confirmar)
| Item | Desligamento (x3) | Risco clínico (x3) | Risco do projeto (x2) | Esforço (x1) | Total | Bloqueado por |
|---|---|---|---|---|---|---|
| <item> | | | | | | |
Decisão do dono do produto: <aprovada/ajustada>, em <data>.
```
