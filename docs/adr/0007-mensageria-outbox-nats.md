# ADR-0007 · Mensageria com outbox transacional e NATS JetStream

- **Status:** proposto
- **Data:** 2026-11-06

## Contexto
Integrações externas falham e demoram; eventos entre módulos e serviços precisam de entrega garantida.

## Decisão
Padrão outbox: o evento é gravado na mesma transação do dado de negócio; um publicador envia ao NATS JetStream. Consumidores são idempotentes, com retentativa exponencial e fila de falhas visível na interface.

## Consequências
Nenhum evento se perde; integrações não travam o atendimento; um componente a mais para operar (NATS, leve).
