mod api;
mod middleware;
mod scheduler;

use std::sync::Arc;

use tracing_subscriber::EnvFilter;

use crate::{
    api::{build_app, default_rate_limit_state, AppState},
    middleware::AuthState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Arc::new(scbdb_core::load_app_config()?);
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(config.log_level.clone()))?;
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let pool_config = scbdb_db::PoolConfig::from_app_config(&config);
    let pool = scbdb_db::connect_pool(&config.database_url, pool_config).await?;
    scbdb_db::run_migrations(&pool).await?;

    let _scheduler = scheduler::build_scheduler(pool.clone(), Arc::clone(&config)).await?;

    let auth = AuthState::from_env(matches!(config.env, scbdb_core::Environment::Development))?;
    let app = build_app(AppState { pool }, auth, default_rate_limit_state());

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("received shutdown signal, starting graceful shutdown");
}
