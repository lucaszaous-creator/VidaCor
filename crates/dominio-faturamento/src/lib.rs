//! Faturamento de convênios (docs/22 E5, docs/21 § 3.9, docs/25 § 3.3), sem I/O (G5).
//!
//! - [`auditoria`]: regras versionadas por operadora (ex.: `AUD-UNIMED-2026.1`) que produzem
//!   pendências; guia com pendência que impede o lote não entra em lote.
//! - [`tiss`]: montagem da guia SP/SADT e do lote no padrão TISS 4.03.00 (XSD oficial da ANS em
//!   `xsd/tiss-4.03.00`), com o hash do epílogo e o SHA-256 do arquivo para o lastro.
//!
//! O núcleo (crate `api`) lê o banco, chama estas funções e grava o resultado com a versão da regra.

pub mod auditoria;
pub mod guia;
pub mod tiss;

pub use auditoria::{auditar, conjunto, ContextoAuditoria, Efeito, Pendencia, CONJUNTOS};
pub use guia::{Centavos, GuiaFaturavel, ItemAutorizado, ItemExecutado, Profissional};
pub use tiss::{montar_lote, ArquivoTiss, ErroTiss, LoteTiss, VERSAO_TISS};
