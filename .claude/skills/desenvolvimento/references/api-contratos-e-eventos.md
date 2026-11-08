# API, contratos e eventos

Fontes: `docs/14-regras-de-arquitetura-e-engenharia.md` A2, A4, A6; `docs/11-arquitetura-e-infraestrutura.md` §6 e §9.2;
`docs/15-padrao-de-qualidade.md` G3; `docs/17-seguranca-por-classe-de-ataque.md` B2, B3, B4, B6; ADR-0007.

## 1. REST

- Recursos no plural, verbos HTTP corretos, **versão no caminho** (`/v1/agendamentos`).
- Toda rota **declara a permissão do ato** (`agenda.agendar`); rota sem declaração não compila ou falha no teste de
  arquitetura (SEC-ACS-1). Negar por padrão.
- Acesso por ID confere se o usuário pode **aquele** objeto (SEC-ACS-2); o filtro vai na consulta (SEC-ACS-3);
  toda rota com ID tem teste de acesso indevido (SEC-ACS-4).
- Paginação por **cursor**, filtros e ordenação explícitos, **limite máximo** de página (SEC-DIS-1, SEC-DIS-4).
- Resposta com dado pessoal ou de saúde: `Cache-Control: no-store` (SEC-AUT-8).
- Dado de saúde nunca na URL (nem em query string); identificadores internos opacos.
- Toda resposta carrega `trace_id` (cabeçalho e corpo de erro) para o suporte localizar o caso (R8).
- Respostas e contagens não revelam se um paciente existe (SEC-DAD-4).

## 2. Erros: Problem Details (RFC 9457)

Código estável para a máquina, mensagem em pt-BR para a tela, no formato R8 do `docs/16`
(o que aconteceu, o que já foi feito, o que fazer agora).

```json
{
  "type": "https://vidacor.local/erros/guia-em-uso",
  "title": "Guia já usada em outro atendimento",
  "status": 409,
  "codigo": "AGENDA_GUIA_EM_USO",
  "detail": "A guia UN-55120 já está ligada ao atendimento ATD-2026-118790. Nada foi agendado. Peça outra guia ao paciente ou à operadora.",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736"
}
```

| Situação | Status |
|---|---|
| Sem permissão para o ato ou o objeto | 403 (sem revelar existência, quando o objeto é sensível) |
| Conflito de recurso limitado (horário, guia, versão) | 409 |
| Regra de negócio violada (saldo zero, senha vencida, valor zero ou negativo) | 422 |
| Formato inválido | 400 |
| Reautenticação necessária (assinar, estornar, desconto, exportar, permissão) | 401 com código próprio de step-up (SEC-AUT-5) |

## 3. Contrato como código

- **OpenAPI gerado do código Rust** (`utoipa`) é a fonte da verdade.
- O cliente TypeScript (`packages/api-client`) é **gerado na CI**: se o contrato quebrar, a tela não compila.
- **Diff de contrato a cada PR** na CI; mudança incompatível só em nova versão (`/v2`).
- Serviços em outras linguagens (JVM, Python) falam com o núcleo só por HTTP/gRPC ou pelo barramento, nunca pelo banco.

## 4. `Idempotency-Key` nas rotas de escrita (G3)

Rotas que criam cobrança, recebimento, guia, agendamento ou laudo aceitam o cabeçalho e devolvem **a mesma resposta**
se a chave se repetir. A chave é UUID gerado pelo cliente **uma vez por operação de negócio** (não por clique).
Nunca dado pessoal como chave.

```sql
CREATE TABLE comum.idempotencia (
  chave          uuid NOT NULL,
  usuario_id     uuid NOT NULL,
  rota           text NOT NULL,
  hash_pedido    bytea NOT NULL,         -- mesma chave com corpo diferente = 422
  status_http    integer,
  resposta       jsonb,
  criado_em      timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (usuario_id, rota, chave)
);
```

Fluxo (pseudocódigo):

```rust
// PSEUDOCÓDIGO
// 1. INSERT ... ON CONFLICT DO NOTHING na mesma transação do negócio.
// 2. Se a linha já existia: comparar hash_pedido; se igual e com resposta, devolver a resposta gravada;
//    se diferente, 422; se ainda em andamento, 409 com orientação para tentar de novo.
// 3. Se é nova: executar o caso de uso, gravar status_http e resposta, COMMIT.
```

Teste obrigatório: "quando o mesmo pedido é enviado duas vezes com a mesma Idempotency-Key, então só um registro existe
e as duas respostas são iguais" (FC-AGD-04).

## 5. Eventos de domínio e outbox (ADR-0007)

- Nome no **passado** (`LaudoAssinado`, `CheckinConcluido`, `RecebimentoRegistrado`), campo `versao`, schema em JSON Schema versionado.
- Consumidores toleram campos novos; valor de enum desconhecido é **rejeitado**, não convertido em padrão (SEC-MSG-2).
- **Outbox transacional**: o evento é gravado na mesma transação do negócio; um relay publica no NATS JetStream.
- Consumidor deduplica por ID do evento, tolera ordem trocada e mensagem velha; **ack só depois de gravar** (SEC-MSG-4).
- Identidade do envelope confere com a do conteúdo (`paciente_id` do evento igual ao do recurso, SEC-MSG-3).
- Cada serviço tem credencial própria no NATS com permissão **só nos assuntos** que usa (SEC-MSG-1).
- Mensagem envenenada vai para a fila de falhas após N tentativas, sem dado de saúde nos diagnósticos (SEC-MSG-5, SEC-DIS-7).

```json
{
  "id": "0192f0c4-7a1e-7c3a-9d2f-1b4e5f6a7b8c",
  "tipo": "LaudoAssinado",
  "versao": 1,
  "ocorrido_em": "2026-11-07T14:32:10Z",
  "unidade_id": "0192f0c4-0000-7000-8000-000000000001",
  "correlacao": "4bf92f3577b34da6a3ce929d0e0e4736",
  "dados": { "laudo_id": "0192...", "versao_laudo": 2, "paciente_id": "0192..." }
}
```

Evento carrega identificadores, não conteúdo clínico. Quem precisa do conteúdo busca pela API, com permissão.

## 6. Tempo real (WebSocket ou SSE)

- Autenticação no handshake, verificação de `Origin`, **autorização por evento**: cada tela recebe só o que pode ver (SEC-CLI-5).
- Eventos tipados, reconexão com retomada a partir do último evento recebido.
- Revogação de acesso vale também para conexões abertas (SEC-DAD-7).
- Uso típico: fila da recepção, imagem chegando do aparelho, laudo assinado, pendências.

## 7. Compatibilidade

- Desktop e servidor negociam versão da API; estação desatualizada funciona por prazo definido (`docs/13` §4).
- Feature flags para liberar por perfil, máquina ou sala (docs/16 C5).
- Mudança de schema de evento: nova `versao`, produtor publica as duas durante a transição quando necessário.
