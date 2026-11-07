# ADR-0018 · Lastro e rastreabilidade em todo registro

- **Status:** proposto
- **Data:** 2026-11-07

## Contexto
A clínica precisa provar, para convênios, auditorias, processos e segurança do paciente, de onde veio cada informação e o que aconteceu com ela. Convênio só é atendido com guia autorizada. Clínicas de imagem rastreiam ainda insumos, dose de radiação, desinfecção de transdutores e manutenção de aparelhos.

## Decisão
Todo registro carrega metadados obrigatórios (id, criador, origem, estação, versão, correlação, hash, regra aplicada, motivo), eventos imutáveis com hash encadeado e ligações tipadas entre registros. O agendamento de convênio exige a seção da guia com conferências automáticas e consumo atômico. Um explorador de lastro e um dossiê assinado mostram a cadeia completa a partir de qualquer código. Detalhes em [docs/21](../21-lastro-e-rastreabilidade.md).

## Consequências
Mais colunas, tabelas de eventos e testes; armazenamento maior. Em troca, defesa de glosa, recall, investigação de incidentes e indicadores passam a se apoiar em dados verificáveis.
