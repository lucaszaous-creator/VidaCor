# Banco, migrations e RLS

Fontes: `docs/14-regras-de-arquitetura-e-engenharia.md` A1, A2, A6, B3; `docs/15-padrao-de-qualidade.md` G2 e G6;
`docs/18-perfis-permissoes-e-rls.md` §7; `docs/21-lastro-e-rastreabilidade.md` §1 e §5; ADR-0009, ADR-0017, ADR-0018.

## 1. Organização

- PostgreSQL 17, **um schema por módulo** (`agenda`, `caixa`, `laudos`...). Nenhum módulo lê tabela de outro.
- **Um papel de banco por módulo**, só com o privilégio necessário, **sem `BYPASSRLS`**. Papel administrativo
  só para migration, fora do horário e auditado.
- Nomes em `snake_case` e em português do domínio; toda FK indexada; sem `SELECT *` no código.
- O núcleo é o **único cliente** do banco. Nada de PostgREST ou API automática (G1).

## 2. Migrations

- Versionadas, **somente aditivas em produção**, em duas etapas (**expand/contract**): adicionar, migrar
  dados e código, remover só em outra versão.
- Lint com `squawk` na CI; lint próprio que **proíbe** `UPDATE`/`DELETE` em tabelas clínicas e de eventos,
  `USING (true)` e `WITH CHECK (true)`.
- **Toda migration nova é revisada por uma pessoa antes de rodar** (G6).
- Nunca apagar dados para "fazer funcionar". Dados duplicados que impedem uma constraint: reportar com
  proposta de limpeza e esperar decisão.
- Migration reversível ou plano de contorno documentado (docs/14 B2, item 6).

## 3. Metadados de lastro em toda tabela (docs/21 §1)

Colunas padrão, verificadas por teste de schema na CI:

```sql
-- Modelo ilustrativo: colunas obrigatórias de lastro
CREATE TABLE agenda.agendamento (
  id               uuid PRIMARY KEY,               -- UUIDv7 gerado pelo núcleo
  codigo           text NOT NULL UNIQUE,           -- legível, ex.: 'AGD-2026-004512'
  criado_em        timestamptz NOT NULL DEFAULT now(),
  criado_por       uuid NOT NULL,                  -- usuário, sistema, aparelho ou integração
  origem           text NOT NULL,                  -- balcao, whatsapp, portal, aparelho, migracao...
  estacao          text NOT NULL,
  unidade_id       uuid NOT NULL,
  versao           integer NOT NULL DEFAULT 1,
  correlacao       text NOT NULL,                  -- trace_id
  regra_aplicada   text,                           -- versão da regra usada no cálculo
  motivo           text,                           -- obrigatório em correção e cancelamento
  -- campos do domínio
  sala_id          uuid NOT NULL,
  inicio           timestamptz NOT NULL,
  paciente_id      uuid NOT NULL,
  situacao         text NOT NULL CHECK (situacao IN ('provisorio','confirmado','cancelado'))
);
```

Além das colunas: **tabela de eventos append-only** por domínio, com hash encadeado, e **tabela de ligações
tipadas** (de, para, tipo, criado_em, criado_por). Identificador externo (StudyUID, protocolo ANS, txid,
código da NFS-e) guardado como recebido, com índice único quando o provedor garante unicidade.

```sql
CREATE TABLE laudos.evento (
  seq          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  agregado_id  uuid NOT NULL,
  tipo         text NOT NULL,          -- 'LaudoAssinado'
  versao       integer NOT NULL,
  dados        jsonb NOT NULL,         -- sem dado de saúde além do necessário ao evento
  hash_anterior bytea NOT NULL,
  hash         bytea NOT NULL,         -- SHA-256(hash_anterior || conteúdo canônico)
  criado_em    timestamptz NOT NULL DEFAULT now(),
  criado_por   uuid NOT NULL
);
-- O papel da aplicação recebe só INSERT e SELECT nesta tabela.
```

## 4. Registro clínico imutável

Laudo, evolução e documento assinado **nunca** recebem `UPDATE` ou `DELETE`. Correção é `INSERT` de nova versão
com `motivo`, e a versão anterior continua visível para quem tem permissão. Garantido em três lugares: papel sem
permissão, ausência de política RLS de escrita e lint de migration.

## 5. Concorrência no banco (G2), nesta ordem

### 5.1 Constraint única (preferida)

```sql
-- Um horário de sala só pode ter um agendamento ativo
CREATE UNIQUE INDEX agendamento_sala_horario_ativo
  ON agenda.agendamento (sala_id, inicio)
  WHERE situacao <> 'cancelado';

-- Uma guia só pode ser consumida por um atendimento
CREATE UNIQUE INDEX guia_consumo_unico ON agenda.consumo_guia (guia_id);
```

A segunda requisição recebe violação de unicidade, que o adaptador traduz para 409 com mensagem R8.

### 5.2 Operação atômica conferindo linhas afetadas

```sql
-- Consome saldo de procedimento da guia sem nunca deixar negativo
UPDATE agenda.guia_procedimento
   SET saldo = saldo - 1
 WHERE guia_id = $1 AND codigo_tuss = $2 AND saldo > 0;
-- O código confere: linhas afetadas = 1, senão 422 "saldo zero" (FC-AGD-07).
```

```rust
// Ilustrativo com sqlx (macro verificada contra o schema em tempo de compilação)
let r = sqlx::query!(
    "UPDATE agenda.guia_procedimento SET saldo = saldo - 1
      WHERE guia_id = $1 AND codigo_tuss = $2 AND saldo > 0",
    guia_id, codigo_tuss.as_str()
)
.execute(&mut *tx)
.await?;
if r.rows_affected() != 1 {
    return Err(ErroAgenda::SaldoEsgotado);
}
```

### 5.3 `SELECT ... FOR UPDATE` quando a regra é mais complexa

```sql
BEGIN;
SELECT id, aberto, operador_id FROM caixa.caixa WHERE id = $1 FOR UPDATE;
-- decidir com a linha travada (caixa aberto e do próprio operador), gravar, COMMIT
```

Proibido lock em memória, lock em Redis com expiração ou decisão baseada em cache. Infraestrutura nova para
exclusividade só com ADR. **Cada proteção tem teste com duas requisições simultâneas** provando que só uma tem efeito.

Recursos limitados conhecidos: horário de sala, senha da fila, caixa aberto, lote de estoque, número de guia e
lote TISS, link de convite, cortesia, versão de laudo, faixas de formulários da contingência (`docs/23` §5).

## 6. RLS (docs/18 §7)

- Ligado **e forçado** (`FORCE ROW LEVEL SECURITY`) em toda tabela com dado N3 ou N4 e nas financeiras.
- Contexto declarado pelo núcleo no início de cada transação, **sempre com `SET LOCAL`** (vale só na transação,
  não vaza entre requisições do pool). Nunca `SET` de sessão.

```sql
SET LOCAL app.usuario_id  = '0192f0c4-...';
SET LOCAL app.perfis      = 'medico';
SET LOCAL app.unidade_id  = '0192f0c4-...';
SET LOCAL app.break_glass = 'false';
```

```rust
// PSEUDOCÓDIGO: set_config(..., true) equivale a SET LOCAL e aceita parâmetro
sqlx::query!("SELECT set_config('app.usuario_id', $1, true)", usuario_id.to_string())
    .execute(&mut *tx).await?;
```

- Proibido `USING (true)`, `WITH CHECK (true)`, `BYPASSRLS` nos papéis da aplicação.
- **Toda política tem teste** com usuário autorizado (vê) e não autorizado (não vê), direto no banco e pela rota.
- Visões derivadas (indicadores, busca, exportação) respeitam o mesmo escopo (SEC-DAD-2).
- Portal na nuvem: RLS por `paciente_id` e por link compartilhado com validade.
- A consulta já carrega o filtro de dono; checar depois de buscar não basta (SEC-ACS-3, SEC-DAD-1).

## 7. Outras restrições

- `CHECK`, `NOT NULL`, `UNIQUE`, FK reforçam as invariantes do domínio; o banco nunca depende só do código.
- Hash SHA-256 de todo arquivo na entrada; conferência periódica de integridade.
- Retenção por tipo de dado: prontuário e imagens 20 anos; financeiro e fiscal pelos prazos legais (docs/21 §5 item 7).
- Testes de lastro: o fluxo crítico grava os eventos e ligações esperados, e repetir não duplica (docs/21 §5 item 9).
