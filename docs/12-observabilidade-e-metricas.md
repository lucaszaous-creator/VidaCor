# 12 · Observabilidade, métricas e SLOs

> **Status:** proposta (v0.1, nov/2026). Complementa a seção 7.2 da
> [11 · Arquitetura e infraestrutura](11-arquitetura-e-infraestrutura.md).
> Decisão registrada no [ADR-0012](adr/0012-observabilidade-opentelemetry-slo.md).

## 1. Princípios

1. **Medir tudo que o usuário sente** (latência, erros, disponibilidade, fluidez da tela) e **tudo o que o negócio precisa** (tempo de laudo, espera na recepção, glosa).
2. **Histogramas, não médias.** A média esconde o paciente que esperou 40 s. Toda latência é registrada como histograma, o que permite calcular **qualquer percentil** depois.
3. **Um padrão só:** OpenTelemetry em todos os componentes (Rust, TypeScript, Python, Kotlin, Orthanc via exportador).
4. **Metas explícitas (SLO)** com **orçamento de erro** e alertas por taxa de consumo, e não por "CPU acima de 80%".
5. **Tudo local:** a stack de observabilidade roda no servidor da clínica e funciona sem internet.

## 2. Percentis

Os percentis são calculados a partir de **histogramas exponenciais** (OpenTelemetry
*exponential histogram* / Prometheus *native histograms*). Com eles, qualquer
quantil pode ser consultado a qualquer momento, sem reconfigurar a coleta:
`histogram_quantile(0.65, …)` devolve o p65; `0.999`, o p99,9.

| Percentil | Para que serve |
|---|---|
| **p50** (mediana) | Experiência típica |
| **p55, p65** | Disponíveis sob demanda, para análise fina da distribuição (quantis consultáveis, não usados em alerta) |
| **p75** | Referência do Google para Core Web Vitals (a tela é "boa" se o p75 estiver dentro da meta) |
| **p90** | Início da cauda: onde aparecem os primeiros problemas |
| **p95** | **Meta principal de SLO** das rotas operacionais |
| **p99** | Cauda longa: consultas lentas, contenção no banco, GC do serviço JVM |
| **p99,9** | Picos raros (abertura de exame grande, lote TISS) |
| **máx.** | Pior caso observado, para investigação |

Além dos percentis: **média, desvio padrão, mínimo/máximo, contagem e taxa por segundo**, além de **heatmaps de latência** (distribuição ao longo do tempo) no Grafana.

## 3. Métricas técnicas

### 3.1 Serviços: RED + os quatro sinais de ouro
Para **cada rota HTTP, consumidor de fila e chamada externa**:

| Métrica | Descrição |
|---|---|
| **Rate** | Requisições por segundo, por rota, método e status |
| **Errors** | Taxa de erro (5xx, timeouts, falhas de negócio classificadas) |
| **Duration** | Histograma de latência → p50…p99,9 |
| **Saturation** | Fila de requisições, conexões abertas, pool do banco ocupado, tarefas `tokio` pendentes |
| **Apdex** | Índice de satisfação (alvo T = 150 ms para o balcão) |

### 3.2 Recursos: USE (utilização, saturação, erros)
CPU, memória, disco (IOPS, latência, fila), rede, **ZFS** (ARC, saúde do pool, *scrub*), **nobreak** (carga, autonomia), temperatura e **SMART** dos discos.

### 3.3 Banco de dados (PostgreSQL)
Latência por consulta (`pg_stat_statements`), conexões e pool, *locks* e *deadlocks*, taxa de acerto do cache, *bloat*, *autovacuum*, **atraso da réplica** (s e bytes), crescimento por schema, idade da transação mais antiga, WAL gerado e arquivado.

### 3.4 PACS e DICOM
| Métrica | Por quê |
|---|---|
| **Latência aparelho → tela** (captura até a imagem aparecer) | Promessa central da tela "Aparelhos" |
| Imagens e estudos recebidos por aparelho, por minuto | Volume e anomalias |
| Falhas de associação DICOM, C-ECHO por aparelho | Aparelho fora do ar |
| Consultas à worklist e tempo de resposta | O aparelho mostra o paciente rápido? |
| MPPS iniciados × concluídos | Exames "esquecidos" abertos |
| Estudos **sem conciliação** (paciente × imagem) | Segurança do paciente |
| Tempo de abertura de estudo no visualizador (p95) | Experiência do médico |
| Disco do PACS, projeção de esgotamento | Capacidade |

### 3.5 Mensageria e integrações
Profundidade de fila e idade da mensagem mais antiga (NATS), eventos no **outbox** sem publicação, taxa de retentativa, **fila de falhas**; por integração externa (TISS, NFS-e, PIX, WhatsApp, Memed, assinatura): latência p95, taxa de erro, disponibilidade do provedor, cota/consumo (mensagens WhatsApp, assinaturas, notas).

### 3.6 Frontend (RUM: medição no navegador real)
| Métrica | Meta (p75) |
|---|---|
| **LCP** (maior conteúdo visível) | ≤ 2,5 s (tela fria) e ≤ 1 s (navegação interna) |
| **INP** (resposta à interação) | ≤ 200 ms (meta interna: ≤ 100 ms no balcão) |
| **CLS** (estabilidade do layout) | ≤ 0,1 |
| **TTFB** | ≤ 200 ms na rede local |
| Erros JavaScript por sessão | Tendência a zero |
| Tempo até a tela ficar utilizável, por tela | Orçamento por tela |
| Latência do WebSocket/SSE (evento → tela) | p95 ≤ 500 ms |

Medido com a biblioteca `web-vitals` + OpenTelemetry Web, enviando para o coletor local. Sem dados pessoais nas métricas.

### 3.7 Disponibilidade e qualidade de entrega
- **Verificações sintéticas** a cada minuto: login, abrir agenda, consultar worklist, gerar PDF de teste, C-ECHO no PACS, portal no ar.
- **DORA**: frequência de deploy, tempo de entrega, taxa de falha de mudança, tempo de restauração.
- **Backup**: idade do último backup bem-sucedido, duração, tamanho, **resultado do teste de restauração**.

## 4. Métricas de negócio e operação clínica

Expostas no Grafana para TI e em **Indicadores** para a gerência.

| Indicador | Definição |
|---|---|
| Tempo de espera na recepção | Chegada → chamada (p50/p90 por período e tipo de senha) |
| Tempo porta → exame | Check-in → MPPS iniciado |
| Tempo de exame | MPPS iniciado → concluído, por tipo de exame e sala |
| **TAT do laudo** (*turnaround time*) | Exame concluído → laudo assinado (p50/p90/p95), por médico, exame e prioridade |
| Laudos fora do SLA | Contagem e percentual |
| Taxa de retificação de laudo | Versões > 1 / total |
| Ocupação por sala e equipamento | Minutos usados / disponíveis |
| No-show e taxa de confirmação | Faltas / agendados; confirmados pelo WhatsApp |
| Glosa | Valor e percentual por convênio e motivo |
| Ciclo de faturamento | Atendimento → lote enviado → pago |
| Caixa | Diferença no fechamento, estornos, descontos autorizados |
| NPS | Por exame, médico e período |
| Estacionamento (fase posterior) | Permanência p50/p90, acerto da leitura de placa |

## 5. SLOs iniciais

| Serviço / jornada | SLI | SLO (janela de 30 dias, horário de funcionamento) |
|---|---|---|
| Telas operacionais (agenda, recepção, caixa) | Latência da API | **p95 ≤ 150 ms**, p99 ≤ 400 ms |
| Busca de paciente | Latência | p95 ≤ 100 ms |
| Abertura de estudo no visualizador | Tempo até a 1ª imagem | p95 ≤ 2 s (TC: ≤ 4 s) |
| Imagem aparelho → tela | Latência ponta a ponta | p95 ≤ 5 s |
| Geração de PDF do laudo | Latência | p95 ≤ 1,5 s |
| Disponibilidade do núcleo | Verificações sintéticas OK | **99,9%** |
| Integrações externas | Entrega concluída em até 15 min | 99,5% (depende do provedor) |
| Portal do paciente | Disponibilidade | 99,5% |

**Orçamento de erro:** 99,9% em ~264 h/mês de funcionamento ≈ **16 min/mês**.
Alertas por **taxa de consumo** (*burn rate*) em duas janelas (rápida 1 h /
lenta 6 h): consumo rápido → página para o plantão; consumo lento → ticket.

## 6. Logs, traces e auditoria

- **Logs estruturados** (JSON) com `trace_id`, sem dados de saúde nem documento do paciente (só identificadores internos).
- **Traces distribuídos** de ponta a ponta: tela → API → banco → fila → integração → Orthanc. Amostragem: 100% dos erros e das requisições lentas (acima do p99) e 10% do restante.
- **Exemplars**: do ponto lento do gráfico de p99 se chega direto ao trace daquela requisição.
- **Auditoria clínica** é separada da observabilidade: fica no banco, imutável, e não é amostrada.

## 7. Stack

| Função | Ferramenta |
|---|---|
| Instrumentação | OpenTelemetry SDK (Rust `tracing-opentelemetry`, JS, Python, Java) |
| Coleta e roteamento | OpenTelemetry Collector |
| Métricas | Prometheus (native histograms) — ou VictoriaMetrics se o volume crescer |
| Logs | Loki |
| Traces | Tempo |
| Painéis e alertas | Grafana + Alertmanager (notificação por e-mail/WhatsApp para o plantão de TI) |
| Sintéticos | Grafana k6 / Blackbox Exporter |
| Perfil contínuo (opcional) | Pyroscope |
| Exportadores | node, postgres, ZFS, SMART, NUT (nobreak), NATS, Orthanc (métricas nativas) |

**Retenção local:** métricas 13 meses (com agregação), logs 30 dias, traces 7 dias.

## 8. Painéis previstos

1. **Visão da clínica agora**: status dos serviços, aparelhos online, filas, SLOs do dia.
2. **API**: RED por rota com heatmap e p50/p75/p90/p95/p99/p99,9.
3. **Banco**: consultas mais lentas, réplica, conexões.
4. **PACS/DICOM**: latência aparelho→tela por sala, falhas por aparelho, estudos sem conciliação.
5. **Integrações**: por provedor, filas e falhas.
6. **Frontend**: Web Vitals por tela e versão.
7. **Infra**: USE, discos, ZFS, nobreak, backups.
8. **Negócio**: TAT de laudo, espera, ocupação, glosa.
9. **SLO e orçamento de erro**.
