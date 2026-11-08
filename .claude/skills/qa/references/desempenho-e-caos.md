# Desempenho, caos e contingência

Fonte: `docs/12-observabilidade-e-metricas.md` § 2, § 3 e § 5, `docs/11` R2, `docs/25` § 3.2 item 6,
`docs/16` C4 item 3 e C6, `docs/23-modo-contingencia.md` § 10 e § 11.

## 1. Orçamento de latência

| Jornada | SLI | Orçamento para o teste de carga |
|---|---|---|
| Telas operacionais (agenda, recepção, caixa) | Latência da API | p95 até 150 ms, p99 até 400 ms |
| Busca de paciente | Latência | p95 até 100 ms |
| Abertura de estudo no visualizador | Tempo até a 1ª imagem | p95 até 2 s (TC até 4 s) |
| Imagem do aparelho até a tela | Ponta a ponta | p95 até 5 s |
| PDF do laudo | Latência | p95 até 1,5 s |
| Integrações externas | Entrega concluída | 99,5% em até 15 min |

Critério de aceite comum de todo módulo: p95 abaixo de 150 ms nas telas operacionais, medido em teste
de carga (`docs/25` § 3.2 item 6).

## 2. Carga de referência

- Volume da PREM-02 (a confirmar): uma unidade, seis salas, cerca de 90 atendimentos por dia.
- Perfil proposto para o teste (a confirmar com o volume real da Fase 0): pico de manhã com 6
  recepções e caixas simultâneos, 6 salas, 4 médicos laudando, busca de paciente a cada poucos
  segundos por estação, mais 3 vezes o pico como margem.
- Banco com volume sintético de vários anos (pacientes, atendimentos, laudos, eventos de lastro) para
  que índices e particionamento sejam exercitados. Nunca dado real.

## 3. Exemplo k6

```js
// pseudocódigo próximo do k6 real; rotas a confirmar no contrato OpenAPI
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  scenarios: {
    balcao: { executor: 'constant-arrival-rate', rate: 30, timeUnit: '1s', duration: '10m', preAllocatedVUs: 50 },
  },
  thresholds: {
    'http_req_duration{rota:agenda_do_dia}': ['p(95)<150', 'p(99)<400'],
    'http_req_duration{rota:busca_paciente}': ['p(95)<100'],
    http_req_failed: ['rate<0.001'],
  },
};

export default function () {
  const h = { headers: { Authorization: `Bearer ${__ENV.TOKEN_TESTE}` } };
  const r = http.get(`${__ENV.BASE}/v1/agenda?dia=2026-11-06&sala=sala-1`, { ...h, tags: { rota: 'agenda_do_dia' } });
  check(r, { '200': (x) => x.status === 200 });
  http.get(`${__ENV.BASE}/v1/pacientes?q=maria%20helena`, { ...h, tags: { rota: 'busca_paciente' } });
}
```

Regras: token de teste de usuário fictício vindo do ambiente (nunca no arquivo); rodar contra
homologação isolada; resultado com p50, p95, p99 e máximo, nunca só média (`docs/12` § 1 item 2);
limiar estourado reprova (não se aumenta o limiar sem decisão registrada).

## 4. Onde roda

- CI: k6 curto nas rotas críticas a cada PR que as toca (`docs/14` B2 item 4, B4).
- Antes do aceite e da prontidão: carga completa em homologação com o perfil do § 2; relatório
  anexado ao relatório de qualidade.
- Achado de latência mostra o trace do ponto lento (exemplars, `docs/12` § 6) e vai para o papel
  `desenvolvimento` com a consulta ou rota responsável.

## 5. Caos leve em homologação (`docs/16` C4 item 3)

A cada versão maior, com o papel `devops-seguranca`. Derrubar de propósito e provar que o atendimento
segue e nada se perde:

| Falha injetada | O que provar |
|---|---|
| PostgreSQL parado ou failover para a réplica | Telas avisam sem mentir (R8); nenhuma escrita confirmada se perde; volta dentro do RTO de 30 min |
| Orthanc parado | Recepção e caixa seguem; estudos chegam depois e são conciliados; nada vai ao laudo sem conciliação |
| NATS parado | Outbox acumula e entrega ao voltar, sem duplicar (G3) |
| Internet fora | PIX, WhatsApp, NFS-e e elegibilidade entram na lista de pendências (R7); reprocessados ao voltar |
| Provedor externo lento ou com erro (fake) | Circuit breaker abre; balcão não fica lento (bulkhead, SEC-DIS-6) |
| Estação perde a rede por segundos | Tela avisa, mantém o digitado e reenvia sem duplicar (R6) |

Cada exercício: hipótese escrita antes, injeção, observação nos painéis, resultado, ações. Relatório
no modelo `docs/modelos/pos-incidente.md`. Nunca em produção sem a aprovação da revisão de mudança, e
nunca com dado real.

## 6. Simulado de contingência (`docs/23` § 10)

Trimestral, alternando prático em homologação e ao vivo em um setor (até 30 min, sem contraste, com
aprovação da coordenação e do responsável clínico). QA prepara:
- massa fictícia com pacientes do dia e fichas numeradas;
- checagem automática dos indicadores do § 11 a partir dos eventos de lastro: kit até 07:00, fichas
  lançadas, divergências de identificação abertas igual a 0, números sem situação igual a 0, estudos
  de contingência sem conciliação após 24 h igual a 0, tempo da volta ao encerramento até 60 min no
  simulado;
- relatório com ações, cada ação virando teste, regra ou treinamento.

## 7. Critérios que reprovam

- p95 ou p99 acima do orçamento em rota operacional;
- erro acima de 0,1% sob carga nominal;
- qualquer perda de dado confirmado ou duplicidade em exercício de caos;
- tela que mostra sucesso quando falhou, ou zero quando não há dado (R8);
- indicador do simulado fora da meta sem ação registrada.
