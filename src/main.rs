use anyhow::Context;
use miracle_backend::{app, config::AppConfig, database, jobs, telemetry};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let config = AppConfig::from_env().context("invalid configuration")?;
    telemetry::init(&config.telemetry);

    // `miracle_backend migrate` applies pending migrations and exits. Deployments
    // run it as a one-off job before rolling out new application instances.
    if std::env::args().nth(1).as_deref() == Some("migrate") {
        let db = database::connect(&config.database)
            .await
            .context("failed to connect to PostgreSQL")?;
        database::run_migrations(&db).await?;
        db.close().await;
        tracing::info!("migrations applied");
        return Ok(());
    }

    let state = app::AppState::connect(config).await?;

    if state.config.database.run_migrations {
        database::run_migrations(&state.db).await?;
    }

    let shutdown = CancellationToken::new();
    tokio::spawn(wait_for_shutdown_signal(shutdown.clone()));

    let worker = state
        .config
        .jobs
        .enabled
        .then(|| tokio::spawn(jobs::worker::run(state.clone(), shutdown.clone())));

    let listener = TcpListener::bind(state.config.server.socket_addr())
        .await
        .context("failed to bind HTTP listener")?;
    tracing::info!(
        address = %listener.local_addr()?,
        environment = %state.config.environment,
        "http server listening"
    );

    axum::serve(listener, app::router(state.clone()))
        .with_graceful_shutdown(shutdown.clone().cancelled_owned())
        .await
        .context("http server error")?;

    // The server may also stop on its own; make sure the worker is told to stop.
    shutdown.cancel();
    if let Some(worker) = worker
        && let Err(error) = worker.await
    {
        tracing::error!(%error, "job worker task failed during shutdown");
    }

    state.db.close().await;
    tracing::info!("shutdown complete");
    Ok(())
}

/// Resolves on Ctrl+C or SIGTERM (sent by container orchestrators), then cancels `token`.
async fn wait_for_shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to listen for Ctrl+C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutdown signal received, draining in-flight requests");
    token.cancel();
}
