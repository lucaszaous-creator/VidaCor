# Métricas de produto

Fontes: `docs/12-observabilidade-e-metricas.md` §4 e §5; `docs/21-lastro-e-rastreabilidade.md` §4;
`docs/26-teste-de-usabilidade.md` §10 e §11; `docs/23-modo-contingencia.md` §11.

## 1. Princípio

Todo épico diz, no documento de design, **como a clínica saberá que deu certo**, com uma
métrica já prevista nos docs. Indicadores são calculados **a partir dos eventos do lastro**,
nunca de campos digitados (`docs/21` §4). Média não é meta: use percentis (p50, p90, p95)
porque a média esconde o paciente que esperou 40 s (`docs/12` §1).

## 2. Métricas de negócio já definidas (`docs/12` §4)

| Indicador | Definição | Épicos que movem |
|---|---|---|
| Tempo de espera na recepção | Chegada até chamada (p50/p90) | Recepção, agenda |
| Tempo porta-exame | Check-in até MPPS iniciado | Recepção, exames |
| Tempo de exame | MPPS iniciado até concluído | Exames |
| TAT do laudo | Exame concluído até laudo assinado (p50/p90/p95) | Laudos |
| Laudos fora do SLA | Contagem e percentual | Laudos |
| Taxa de retificação | Versões maiores que 1 sobre o total | Laudos, laudo estruturado |
| Ocupação por sala e equipamento | Minutos usados sobre disponíveis | Agenda, exames |
| No-show e confirmação | Faltas sobre agendados; confirmados pelo WhatsApp | Agenda, comunicação |
| Glosa | Valor e percentual por convênio e motivo | Faturamento, autorização, auditoria |
| Ciclo de faturamento | Atendimento até lote enviado até pago | Faturamento, financeiro |
| Caixa | Diferença no fechamento, estornos, descontos autorizados | Caixa |
| NPS | Por exame, médico e período | Comunicação |

## 3. Metas de serviço que o produto promete (`docs/12` §5)

Telas operacionais p95 até 150 ms; busca de paciente p95 até 100 ms; primeira imagem no
visualizador p95 até 2 s (TC até 4 s); imagem do aparelho até a tela p95 até 5 s; PDF do
laudo p95 até 1,5 s; disponibilidade do núcleo 99,9% no horário de funcionamento; portal
99,5%. São metas do contrato (`docs/25` §7.1); o PO não promete valor diferente.

## 4. Métricas de usabilidade (`docs/26`)

Sucesso por tarefa, tempo até 1,5 vez o tempo-alvo, erros críticos (zero), pedidos de
ajuda, SUS médio de pelo menos 75 por perfil. São critério de aceite, não apenas indicador.

## 5. Como escolher a métrica de um épico

1. Partir do problema escrito no comunicado de trás para frente.
2. Escolher **uma** métrica principal da tabela do § 2 (ou do § 3, se o problema é tempo de resposta).
3. Registrar a **linha de base**: como é hoje na Medware/Medilab, quando houver dado; se não houver, escrever "linha de base não verificada" e combinar a medição nas primeiras semanas de piloto.
4. Definir a meta com o dono do produto, marcada "a confirmar" até a aprovação.
5. Dizer de qual evento do lastro a métrica é calculada.

## 6. Proteções na métrica

- Nenhuma métrica identifica paciente: só agregados (`docs/18`, gerente sem acesso individual por padrão).
- Métrica por médico (TAT, retificação) é usada de forma educativa e confidencial, como na revisão por pares (I2), nunca exposta em painel geral sem decisão da direção.
- Indicador derivado respeita as mesmas permissões da origem (SEC-DAD-2).
- Dado de saúde nunca em métrica (SEC-DAD-3).

## 7. Métricas de contingência e migração

Dois épicos transversais têm indicadores próprios, definidos nos seus documentos, e não
devem ganhar métrica paralela:
- Contingência: indicadores do `docs/23` §11 (tempo até declarar, retomada em até 60 min no simulado, papéis numerados lançados e conciliados).
- Migração: critérios numéricos do `docs/24` §7.2 (amostra de 400 cadastros com zero erro de identificação, 300 laudos, 1% das imagens com mínimo de 200) e critério de virada do §8.2.

## 8. Revisão das métricas

- No comitê mensal, o relatório traz as métricas principais dos épicos já em produção, com percentil e tendência.
- Métrica que piorou depois de um lançamento é tratada como possível defeito: o PO abre item no backlog e pede análise ao `qa` e ao `devops-seguranca` (comparação canário, `docs/16` C5.3).
- Mudar a definição de uma métrica é decisão registrada em `docs/decisoes.md`, para que a série histórica continue comparável.

## 9. Modelo

```markdown
## Métrica de sucesso do épico <nome>
- Problema: <frase do comunicado>
- Métrica principal: <indicador do docs/12 §4> | Percentil: <p50/p90/p95>
- Fonte no lastro: <eventos de início e fim>
- Linha de base: <valor e origem> ou "não verificada" (medir de DD/MM a DD/MM)
- Meta: <valor> (a confirmar pelo dono do produto)
- Métricas de proteção: <ex.: taxa de retificação não piora; erro crítico zero>
- Painel: <Indicadores ou Grafana, painel 8 "Negócio">
```

Exemplo: épico Laudos. Métrica principal TAT do laudo p90, do evento de MPPS concluído ao
evento `LaudoAssinado`. Linha de base não verificada (a Medilab não foi medida). Meta a
confirmar. Proteções: taxa de retificação não aumenta; nenhum laudo assinado de paciente
errado (RC-001).
