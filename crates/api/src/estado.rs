//! Estado compartilhado pelas rotas.

use crate::config::Config;
use crate::eventos::EventoTela;
use crate::oidc::Oidc;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::{broadcast, Notify};

#[derive(Clone)]
pub struct Estado {
    pub banco: PgPool,
    pub config: Arc<Config>,
    /// Eventos para as telas (SSE), filtrados por unidade e perfil na entrega.
    pub eventos: broadcast::Sender<EventoTela>,
    /// Acorda o trabalhador do outbox logo depois de um commit.
    pub outbox_aviso: Arc<Notify>,
    pub oidc: Option<Arc<Oidc>>,
}

impl Estado {
    #[must_use]
    pub fn novo(banco: PgPool, config: Config, oidc: Option<Oidc>) -> Self {
        let (eventos, _) = broadcast::channel(256);
        Self {
            banco,
            config: Arc::new(config),
            eventos,
            outbox_aviso: Arc::new(Notify::new()),
            oidc: oidc.map(Arc::new),
        }
    }
}
