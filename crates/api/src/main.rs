//! Binário do núcleo VidaCor.

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use vidacor_api::config::Config;
use vidacor_api::oidc::Oidc;
use vidacor_api::orthanc::Orthanc;
use vidacor_api::{aplicacao, outbox, Estado};

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--openapi") {
        println!("{}", vidacor_api::openapi::json()?);
        return Ok(());
    }

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let config = Config::do_ambiente()?;

    if let Some(admin) = &config.banco_admin_url {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(admin)
            .await?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("migrations")?;
        pool.close().await;
        tracing::info!("migrations aplicadas");
    }

    let banco = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.banco_url)
        .await
        .context("conexão com o banco")?;

    let oidc = match &config.oidc {
        Some(cfg) => match Oidc::descobrir(cfg, &config.url_api).await {
            Ok(o) => Some(o),
            Err(e) => {
                tracing::error!(erro = %e, "provedor de identidade indisponível; login desligado");
                None
            }
        },
        None => None,
    };

    let endereco = config.endereco;
    let orthanc_url = config.orthanc_url.clone();
    let estado = Estado::novo(banco, config, oidc);

    if let Some(url) = orthanc_url {
        let orthanc = Orthanc::novo(&url)?;
        tokio::spawn(outbox::rodar(estado.clone(), orthanc));
    }

    let ouvinte = tokio::net::TcpListener::bind(endereco).await?;
    tracing::info!(%endereco, "núcleo VidaCor no ar");
    axum::serve(ouvinte, aplicacao(estado))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
