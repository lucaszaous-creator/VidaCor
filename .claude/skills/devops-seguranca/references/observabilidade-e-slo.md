# Observabilidade, SLO e alertas

Fontes: `docs/12-observabilidade-e-metricas.md`, `docs/16-protecao-do-usuario.md` C6, `docs/14-regras-de-arquitetura-e-engenharia.md` A5,
ADR-0012.

## 1. Princípios

1. Medir o que o usuário sente (latência, erro, disponibilidade, fluidez da tela) e o que o negócio
   precisa (TAT do laudo, espera, glosa).
2. **Histogramas, não médias.** Toda latência é histograma exponencial; qualquer percentil é
   consultável depois.
3. Um padrão só: OpenTelemetry em Rust, TypeScript, Python, Kotlin e Orthanc via exportador.
4. Metas explícitas (SLO) com orçamento de erro e alerta por **taxa de consumo**.
5. Tudo local: a stack roda no servidor da clínica e funciona sem internet.

## 2. O que medir

| Camada | Métricas |
|---|---|
| Serviços (RED) | Taxa, erros, duração por rota, consumidor de fila e chamada externa; saturação (pool do banco, tarefas `tokio`); Apdex com T = 150 ms |
| Recursos (USE) | CPU, memória, disco (IOPS, latência, fila), rede, ZFS (ARC, saúde, *scrub*), nobreak, SMART, temperatura |
| PostgreSQL | `pg_stat_statements`, conexões, *locks*, *deadlocks*, cache, *bloat*, *autovacuum*, atraso da réplica, WAL arquivado, transação mais antiga |
| PACS e DICOM | Latência aparelho até tela, falhas de associação, C-ECHO por aparelho, worklist, MPPS iniciados contra concluídos, **estudos sem conciliação**, abertura no visualizador, disco do PACS |
| Mensageria | Fila e idade da mensagem mais antiga no NATS, outbox sem publicação, retentativas, fila de falhas, por integração externa |
| Frontend (RUM) | LCP, INP, CLS, TTFB, erros JS, latência WebSocket/SSE; sem dado pessoal |
| Entrega | Sintéticos a cada minuto (login, agenda, worklist, PDF de teste, C-ECHO, portal), DORA, idade do último backup e resultado da restauração |
| Negócio | Espera na recepção, porta até exame, TAT do laudo, laudos fora do SLA, retificação, ocupação, no-show, glosa, diferença de caixa |

## 3. SLOs iniciais (docs/12 §5)

Janela de 30 dias, horário de funcionamento.

| Jornada | SLI | SLO |
|---|---|---|
| Telas operacionais | Latência da API | p95 até 150 ms, p99 até 400 ms |
| Busca de paciente | Latência | p95 até 100 ms |
| Visualizador | Tempo até a 1ª imagem | p95 até 2 s (TC até 4 s) |
| Imagem aparelho até tela | Latência ponta a ponta | p95 até 5 s |
| PDF do laudo | Latência | p95 até 1,5 s |
| Núcleo | Sintéticos OK | 99,9% |
| Integrações externas | Entrega em até 15 min | 99,5% |
| Portal | Disponibilidade | 99,5% |

**Orçamento de erro:** 99,9% em cerca de 264 h de funcionamento por mês dá cerca de **16 minutos por mês**.
Orçamento estourado: só correções até normalizar (docs/16 C6). O SLO de 99,9% também é o compromisso
contratual (docs/25 §7.1).

## 4. Alertas por taxa de consumo

Duas janelas (docs/12 §5): consumo rápido em 1 h pagina o plantão; consumo lento em 6 h abre chamado.
Esboço de regras Prometheus para o SLO de disponibilidade de 99,9% (nomes de métrica a confirmar com a
instrumentação real; o fator 14,4 corresponde a consumir 2% do orçamento mensal em 1 hora, o fator 6 a 5%
em 6 horas):

```yaml
groups:
  - name: slo-nucleo-disponibilidade
    rules:
      - record: slo:erro_ratio:taxa1h
        expr: |
          sum(rate(http_server_requests_total{servico="nucleo",classe="5xx"}[1h]))
          /
          sum(rate(http_server_requests_total{servico="nucleo"}[1h]))
      - record: slo:erro_ratio:taxa6h
        expr: |
          sum(rate(http_server_requests_total{servico="nucleo",classe="5xx"}[6h]))
          /
          sum(rate(http_server_requests_total{servico="nucleo"}[6h]))

      - alert: NucleoConsumoRapidoOrcamento
        expr: |
          slo:erro_ratio:taxa1h > (14.4 * 0.001)
          and
          sum(rate(http_server_requests_total{servico="nucleo",classe="5xx"}[5m]))
            / sum(rate(http_server_requests_total{servico="nucleo"}[5m])) > (14.4 * 0.001)
        labels: { gravidade: pagina }
        annotations:
          resumo: "Núcleo consumindo o orçamento de erro rápido demais"
          runbook: "infra/runbooks/nucleo.md"

      - alert: NucleoConsumoLentoOrcamento
        expr: |
          slo:erro_ratio:taxa6h > (6 * 0.001)
          and
          slo:erro_ratio:taxa1h > (6 * 0.001)
        labels: { gravidade: chamado }
        annotations:
          resumo: "Núcleo consumindo o orçamento de erro acima do previsto"
          runbook: "infra/runbooks/nucleo.md"
```

Regras para todo alerta:
- Tem dono, runbook e ação clara. Alerta sem ação vira painel, não alerta.
- Nunca alertar por "CPU acima de 80%" isolado; recursos entram como sinal de saturação ligado a SLO
  ou a projeção de esgotamento (disco do PACS, WAL, backup atrasado).
- Alertas de segurança do paciente (estudos sem conciliação, MPPS aberto além do prazo) seguem como
  alerta operacional com dono clínico informado.
- Notificação por e-mail ou WhatsApp para o plantão de TI pelo Alertmanager (canal **a confirmar**).

## 5. Logs, traces e auditoria

- Logs estruturados em JSON com `trace_id`, **sem dado de saúde, CPF, nome ou documento**; só
  identificadores internos (SEC-DAD-3).
- Traces de ponta a ponta (tela, API, banco, fila, integração, Orthanc). Amostragem: 100% dos erros e das
  requisições acima do p99, 10% do restante. Exemplars ligam o ponto lento ao trace.
- Atributos de span revisados: nenhum corpo de requisição, nenhum parâmetro de consulta com dado
  pessoal, nenhum texto de laudo.
- Toda resposta carrega `trace_id`, que é o código de suporte mostrado ao usuário (docs/16 R8).
- **Auditoria clínica é separada**: fica no banco, imutável, sem amostragem e com as permissões do
  docs/18. Observabilidade não é leitor alternativo de dado clínico.

## 6. Stack e retenção (docs/12 §7)

| Função | Ferramenta |
|---|---|
| Instrumentação | OpenTelemetry SDK (Rust `tracing-opentelemetry`, JS, Python, Java) |
| Coleta | OpenTelemetry Collector (também remove atributos proibidos antes de gravar) |
| Métricas | Prometheus com native histograms (VictoriaMetrics se o volume crescer) |
| Logs | Loki |
| Traces | Tempo |
| Painéis e alertas | Grafana e Alertmanager, só na VLAN de administração |
| Sintéticos | k6 e Blackbox Exporter |
| Exportadores | node, postgres, ZFS, SMART, NUT, NATS, Orthanc |

Retenção local: métricas 13 meses (com agregação), logs 30 dias, traces 7 dias.

## 7. Painéis previstos (docs/12 §8)

Visão da clínica agora; API (RED com heatmap e percentis); banco; PACS e DICOM; integrações; frontend;
infra; negócio; SLO e orçamento de erro. Painel de negócio para a gerência mostra agregados, nunca dado
individual de paciente (docs/18 §4).

## 8. Checklist de instrumentação de serviço novo

- [ ] Histogramas de duração por rota e por chamada externa
- [ ] Contadores de erro classificados (5xx, timeout, falha de negócio)
- [ ] Saturação exposta (pool, fila, tarefas pendentes)
- [ ] Health de prontidão e vivacidade
- [ ] Logs JSON com `trace_id` e filtro de atributos no Collector
- [ ] SLO, painel e alerta de taxa de consumo com runbook
- [ ] Revisão: nenhum dado de saúde em log, métrica, trace ou URL
