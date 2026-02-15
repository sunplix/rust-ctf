mod auth;
mod config;
mod error;
mod routes;
mod state;

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use config::AppConfig;
use state::AppState;
use tokio::signal;
use tokio::time::{sleep, Duration, MissedTickBehavior};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{info, warn, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = AppConfig::from_env()?;
    let state = Arc::new(AppState::new(config.clone()).await?);
    spawn_runtime_alert_scanner(Arc::clone(&state));
    spawn_instance_reaper(Arc::clone(&state));

    let app = build_router(state);
    let addr: SocketAddr = format!("{}:{}", config.app_host, config.app_port).parse()?;

    info!("rust-ctf backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(routes::router())
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(false),
                )
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                )
                .on_failure(
                    DefaultOnFailure::new()
                        .level(Level::ERROR)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .layer(CorsLayer::permissive())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "rust_ctf_backend=info,tower_http=info,axum::rejection=trace".into()
            }),
        )
        .compact()
        .init();
}

fn spawn_runtime_alert_scanner(state: Arc<AppState>) {
    if !state.config.runtime_alert_scan_enabled {
        info!("runtime alert scanner disabled by configuration");
        return;
    }

    let interval_seconds = state
        .config
        .runtime_alert_scan_interval_seconds
        .clamp(10, 3600);
    let initial_delay_seconds = state
        .config
        .runtime_alert_scan_initial_delay_seconds
        .min(3600);

    info!(
        interval_seconds,
        initial_delay_seconds, "runtime alert scanner task scheduled"
    );

    tokio::spawn(async move {
        if initial_delay_seconds > 0 {
            sleep(Duration::from_secs(initial_delay_seconds)).await;
        }

        let mut ticker = tokio::time::interval(Duration::from_secs(interval_seconds));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            match routes::admin::run_runtime_alert_scan(state.as_ref()).await {
                Ok(summary) => {
                    info!(
                        upserted = summary.upserted,
                        auto_resolved = summary.auto_resolved,
                        open_count = summary.open_count,
                        acknowledged_count = summary.acknowledged_count,
                        resolved_count = summary.resolved_count,
                        "runtime alert scanner tick completed"
                    );
                }
                Err(err) => {
                    warn!(error = %err, "runtime alert scanner tick failed");
                }
            }
        }
    });
}

fn spawn_instance_reaper(state: Arc<AppState>) {
    if !state.config.instance_reaper_enabled {
        info!("instance reaper disabled by configuration");
        return;
    }

    let interval_seconds = state
        .config
        .instance_reaper_interval_seconds
        .clamp(10, 3600);
    let initial_delay_seconds = state.config.instance_reaper_initial_delay_seconds.min(3600);
    let batch_size = state.config.instance_reaper_batch_size.clamp(1, 500);
    let stale_reaper_enabled = state.config.instance_stale_reaper_enabled;
    let heartbeat_stale_seconds = state
        .config
        .instance_heartbeat_stale_seconds
        .clamp(60, 86_400);
    let stale_reaper_batch_size = state.config.instance_stale_reaper_batch_size.clamp(1, 500);

    info!(
        interval_seconds,
        initial_delay_seconds,
        batch_size,
        stale_reaper_enabled,
        heartbeat_stale_seconds,
        stale_reaper_batch_size,
        "instance reaper task scheduled"
    );

    tokio::spawn(async move {
        if initial_delay_seconds > 0 {
            sleep(Duration::from_secs(initial_delay_seconds)).await;
        }

        let mut ticker = tokio::time::interval(Duration::from_secs(interval_seconds));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;
            match routes::instances::run_expired_instance_reaper(state.as_ref(), batch_size).await {
                Ok(summary) => {
                    info!(
                        scanned = summary.scanned,
                        reaped = summary.reaped,
                        failed = summary.failed,
                        skipped = summary.skipped,
                        batch_size,
                        "instance reaper tick completed"
                    );
                }
                Err(err) => {
                    warn!(error = %err, "instance reaper tick failed");
                }
            }

            if stale_reaper_enabled {
                match routes::instances::run_stale_instance_reaper(
                    state.as_ref(),
                    heartbeat_stale_seconds as i64,
                    stale_reaper_batch_size,
                )
                .await
                {
                    Ok(summary) => {
                        info!(
                            scanned = summary.scanned,
                            reaped = summary.reaped,
                            failed = summary.failed,
                            skipped = summary.skipped,
                            heartbeat_stale_seconds,
                            stale_reaper_batch_size,
                            "stale instance reaper tick completed"
                        );
                    }
                    Err(err) => {
                        warn!(error = %err, "stale instance reaper tick failed");
                    }
                }
            }
        }
    });
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("shutdown signal received");
}
