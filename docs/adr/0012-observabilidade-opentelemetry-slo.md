# ADR-0012 · Observabilidade com OpenTelemetry, histogramas e SLOs

- **Status:** aceito (aprovado pelo dono do produto em 2026-11-08)
- **Data:** 2026-11-07

## Contexto
O time quer métricas robustas (p50, p55, p65, p75, p90, p95, p99, p99,9 e outras), operação local sem internet e diagnóstico rápido de lentidão em um sistema poliglota.

## Decisão
OpenTelemetry em todos os componentes; latências registradas como histogramas exponenciais (Prometheus native histograms), permitindo calcular qualquer percentil sob demanda. Stack local: OTel Collector, Prometheus, Loki, Tempo, Grafana e Alertmanager. SLOs com orçamento de erro e alertas por taxa de consumo. Medição de Web Vitals no navegador (RUM). Detalhes em [12](../12-observabilidade-e-metricas.md).

## Consequências
Qualquer quantil disponível sem reconfigurar coleta; correlação métrica → trace → log; custo de operar a stack de observabilidade no servidor local (CPU, disco, retenção definida).
