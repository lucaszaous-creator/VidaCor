# Backend em Rust

Fontes: `docs/11-arquitetura-e-infraestrutura.md` §4, §5, §9; `docs/14-regras-de-arquitetura-e-engenharia.md`
A1 a A5, B3; ADR-0002 e ADR-0004.

## 1. Forma do núcleo

- **Monólito modular** (ADR-0004): um binário `crates/api` (axum) compõe os módulos. Microserviço só
  existe quando a tecnologia ou o local é outro (Orthanc, JVM, Python, portal na nuvem).
- **Um crate por bounded context**: `dominio-identidade`, `dominio-pacientes`, `dominio-agenda`,
  `dominio-recepcao`, `dominio-caixa`, `dominio-atendimento`, `dominio-exames`, `dominio-laudos`,
  `dominio-faturamento`, `dominio-financeiro`, `dominio-estoque`, `dominio-comunicacao`,
  `dominio-auditoria`, `dominio-indicadores` (lista do `docs/11` §5).
- Cada módulo é dono das **suas tabelas** (schema próprio), expõe uma interface Rust (trait) e publica
  **eventos de domínio** no passado (`CheckinConcluido`, `EstudoRecebido`, `LaudoAssinado`).
- **Nenhum módulo lê tabela de outro** (verificado por teste de arquitetura na CI).
- Linguagem ubíqua: código em português do domínio (`Guia`, `Laudo`, `Encaixe`, `Glosa`); termos
  técnicos consagrados em inglês. Glossário em `docs/glossario.md` (a criar quando houver o primeiro módulo).

## 2. Camadas (arquitetura hexagonal)

| Camada | Pode | Não pode |
|---|---|---|
| `dominio-*` | Agregados, invariantes, cálculos, traits (portas), erros de domínio | Conhecer axum, sqlx, HTTP, Orthanc, NATS, provedores, relógio do sistema |
| Aplicação (casos de uso no próprio crate ou em `api`) | Orquestrar transação, chamar portas, publicar no outbox | Regra de negócio espalhada |
| Adaptadores | Implementar portas com sqlx, Orthanc, reqwest, NATS | Decidir regra de negócio |
| `api` | Rotas, autenticação, autorização do ato, Problem Details, OpenAPI | Lógica de domínio |

Exemplo de porta e regra pura (ilustrativo):

```rust
// crates/dominio-caixa/src/lib.rs
#![forbid(unsafe_code)]

pub struct Centavos(i64);

#[derive(Debug, thiserror::Error)]
pub enum ErroCaixa {
    #[error("desconto acima do limite do perfil")]
    DescontoAcimaDoLimite,
    #[error("valor deve ser maior que zero")]
    ValorInvalido,
    #[error("estouro aritmético")]
    Estouro,
}

/// Porta: o domínio pede, o adaptador implementa.
pub trait TabelaDePrecos {
    fn preco(&self, procedimento: &CodigoTuss) -> Result<(Centavos, VersaoRegra), ErroCaixa>;
}

/// Regra pura: o total é sempre recalculado no servidor (G1).
pub fn total(itens: &[Centavos], desconto: Centavos, limite: Centavos) -> Result<Centavos, ErroCaixa> {
    if desconto.0 > limite.0 {
        return Err(ErroCaixa::DescontoAcimaDoLimite);
    }
    let bruto = itens
        .iter()
        .try_fold(0_i64, |acc, c| acc.checked_add(c.0))
        .ok_or(ErroCaixa::Estouro)?;
    let liquido = bruto.checked_sub(desconto.0).ok_or(ErroCaixa::Estouro)?;
    if liquido <= 0 {
        return Err(ErroCaixa::ValorInvalido);
    }
    Ok(Centavos(liquido))
}
```

## 3. Tipos de valor (docs/14 A3)

| Tipo | Regra |
|---|---|
| Dinheiro | `i64` em centavos (ou `Decimal`); nunca `f32`/`f64`; aritmética `checked_*` (SEC-MEM-2) |
| Data e hora | `timestamptz` em UTC; exibir em `America/Sao_Paulo`; nascimento como `date` |
| Identificador | UUIDv7 como chave; código legível à parte (`ATD-2026-118790`, `ACC 2026-118790`) |
| Documento brasileiro | Tipos validados: `Cpf`, `Cnpj`, `Cns`, `Placa`, `CodigoTuss`, `Cid10`; nunca `String` solta |
| Texto | UTF-8, normalização Unicode na busca |
| Unidade clínica | Valor com unidade (`mm`, `%`, `mmHg`) |

Construtor que valida é a única porta de entrada do tipo:

```rust
pub struct Cpf(String); // só dígitos, já validado

impl Cpf {
    pub fn novo(entrada: &str) -> Result<Self, ErroDocumento> {
        let digitos: String = entrada.chars().filter(char::is_ascii_digit).collect();
        if digitos.len() != 11 || !digitos_verificadores_ok(&digitos) {
            return Err(ErroDocumento::CpfInvalido);
        }
        Ok(Self(digitos))
    }
}
```

## 4. Erros

- Domínio: `thiserror` com variantes nomeadas e estáveis. Borda (`api`, binários): `anyhow` só para
  contexto de falha de infraestrutura.
- Cada erro de domínio mapeia para um **Problem Details** (RFC 9457) com código estável e mensagem em
  pt-BR no formato R8 (o que aconteceu, o que foi feito, o que fazer). Ver `api-contratos-e-eventos.md`.
- Proibido `unwrap()`, `expect()`, `panic!` em código de produção (só em teste), verificado por clippy.
- Parser (DICOM, XML, retorno de convênio) nunca entra em pânico com entrada malformada (SEC-MEM-3).

## 5. Transações e efeitos

- **Transações curtas**. Nada de chamada de rede dentro de transação.
- Efeito externo (PIX, NFS-e, WhatsApp, lote TISS, publicação no portal) sai pelo **outbox**: grava-se o
  evento na mesma transação do negócio e um worker entrega com retentativa e idempotência.
- Fluxo longo (check-in, guia, cobrança, exame) como **saga** com compensação.
- Agregado editado por várias pessoas (agenda, caixa): **concorrência otimista** por coluna `versao`.
- Início de cada transação declara o contexto RLS com `SET LOCAL app.*` (ver `banco-migrations-e-rls.md`).

Esqueleto de caso de uso (pseudocódigo marcado, a confirmar na implementação):

```rust
// PSEUDOCÓDIGO: forma do caso de uso, não a API exata das bibliotecas
async fn receber(ctx: &Contexto, cmd: Receber) -> Result<Recebimento, ErroApp> {
    ctx.exigir_ato("caixa.receber")?;                 // permissão do ato (SEC-ACS-1)
    let mut tx = ctx.db.begin().await?;
    declarar_contexto_rls(&mut tx, ctx).await?;        // SET LOCAL app.*
    let caixa = caixas::aberto_do_operador(&mut tx, ctx.usuario_id).await?; // filtro de dono na consulta
    let total = dominio_caixa::total(&cmd.itens_recalculados, cmd.desconto, ctx.limite_desconto)?;
    let rec = recebimentos::inserir(&mut tx, &caixa, total, &ctx.lastro()).await?;
    outbox::publicar(&mut tx, Evento::RecebimentoRegistrado { id: rec.id, versao: 1 }).await?;
    tx.commit().await?;
    Ok(rec)                                            // cobrança PIX sai pelo outbox, fora da transação
}
```

## 6. Resiliência (docs/14 A5)

- Timeout em toda chamada de rede; nenhuma espera infinita.
- Retentativa com backoff exponencial e jitter só para operação idempotente; teto de tentativas e fila
  de falhas (SEC-DIS-7).
- Circuit breaker por integração e pools separados (bulkhead), para que uma integração lenta não pare o
  balcão (SEC-DIS-6).
- Degradação graciosa: sem internet o atendimento continua e o que depende de fora vira pendência (R7).
- Health checks de prontidão e vivacidade.

## 7. Assíncrono e cancelamento

- `tokio`: tarefas canceladas não continuam em segundo plano; recursos liberados (SEC-DIS-8).
- Não segurar lock nem transação através de `await` em chamada externa.
- Limites de corpo, upload, página, profundidade e tempo por requisição na borda (SEC-DIS-1); regex só
  com o crate `regex` (linear, SEC-DIS-2).

## 8. Qualidade

- `rustfmt`, `clippy -D warnings` com grupo `pedantic` selecionado, `cargo-nextest`, `sqlx` offline na CI.
- `#![forbid(unsafe_code)]` nos crates de domínio; `unsafe` só em crate de FFI revisado (SEC-MEM-1).
- Sem `clone()` para contornar o borrow checker sem justificativa; funções curtas; comentário explica o porquê.
- Cobertura mínima 80% nos domínios e 90% em laudos, faturamento, caixa e assinatura; `cargo-mutants` nos
  críticos; `proptest` em regras combinatórias; `cargo-fuzz` nos parsers (docs/14 B4).
- Orçamento de latência: p95 até 150 ms nas rotas operacionais (docs/12 §5).
