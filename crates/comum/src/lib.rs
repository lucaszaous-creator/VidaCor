//! Tipos comuns do núcleo VidaCor. Sem I/O: pode ser usado pelos crates de domínio (G5).

#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]

pub mod ids;
pub mod problema;
pub mod tipos;

pub use ids::{novo_id, CodigoLegivel};
pub use problema::Problema;
pub use tipos::{DataNascimento, ErroTipo, NomeCompleto};
