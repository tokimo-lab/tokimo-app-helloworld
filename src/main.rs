//! Reference Tokimo app.
//!
//! Demonstrates:
//! - Connecting to the broker via `ClientConfig::from_env()` (reads
//!   `TOKIMO_BUS_SOCKET` + `TOKIMO_BUS_TOKEN` injected by the supervisor).
//! - Declaring methods (`echo`, `greet`) that the broker can invoke.
//! - Publishing periodic events on `helloworld.heartbeat`.
//! - Shutting down gracefully on SIGINT / Shutdown frame.

use std::{sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use tokimo_bus_client::{BusClient, ClientConfig};
use tokimo_bus_protocol::{BusError, MethodDecl};
use tracing::{error, info};

#[derive(Deserialize)]
struct GreetRequest {
    name: String,
}

#[derive(Serialize)]
struct GreetResponse {
    message: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tokimo_bus_client=info".into()),
        )
        .init();

    if let Err(e) = run().await {
        error!(error = %e, "helloworld: fatal");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), BusError> {
    let cfg = ClientConfig::from_env()?;
    info!(endpoint = ?cfg.endpoint, "helloworld: connecting");

    let client = BusClient::builder(cfg)
        .service("helloworld", env!("CARGO_PKG_VERSION"))
        .method(MethodDecl {
            name: "echo".into(),
            requires_auth: false,
            streaming: false,
            description: Some("Returns the request payload unchanged.".into()),
        })
        .method(MethodDecl {
            name: "greet".into(),
            requires_auth: false,
            streaming: false,
            description: Some("Returns a JSON greeting for `{ name }`.".into()),
        })
        .on_invoke("echo", |req| async move { Ok(req.payload) })
        .on_invoke("greet", |req| async move {
            let GreetRequest { name } = serde_json::from_slice(&req.payload)
                .map_err(|e| BusError::BadRequest(e.to_string()))?;
            let resp = GreetResponse {
                message: format!("Hello, {name}!"),
            };
            serde_json::to_vec(&resp).map_err(|e| BusError::Internal(e.to_string()))
        })
        .publishes("helloworld.heartbeat")
        .build()
        .await?;

    info!("helloworld: registered with broker");

    let heartbeat = tokio::spawn(heartbeat_loop(client.clone()));
    let shutdown = tokio::spawn({
        let client = client.clone();
        async move {
            client.run_until_shutdown().await;
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("helloworld: SIGINT received");
            client.shutdown();
        }
        _ = shutdown => {
            info!("helloworld: broker sent Shutdown");
        }
    }

    heartbeat.abort();
    Ok(())
}

async fn heartbeat_loop(client: Arc<BusClient>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(30));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let payload = serde_json::json!({
            "service": client.service_name(),
            "ts": unix_ms(),
        });
        if let Err(e) = client
            .publish(
                "helloworld.heartbeat",
                serde_json::to_vec(&payload).unwrap_or_default(),
            )
            .await
        {
            tracing::warn!(error = %e, "heartbeat publish failed");
        }
    }
}

fn unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
