//! Helloworld pilot app.
//!
//! Demonstrates the full Tokimo multi-process app contract:
//! - Connects to the broker via `TOKIMO_BUS_*` env vars
//! - Bootstraps its own PG schema (`DB_SCHEMA`) + runs embedded migrations
//! - Exposes CRUD methods for `items` table
//! - Calls `notification_center.notify` cross-app via bus
//! - Serves embedded UI assets (with `TOKIMO_APP_ASSETS_DIR` dev override)

mod assets;
mod data_plane;
mod db;
mod handlers;

use std::sync::{Arc, OnceLock};

use tokimo_bus_client::{BusClient, ClientConfig};
use tokimo_bus_protocol::{HttpMethod, MethodDecl};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tokimo_bus_client=info,tokimo_app_helloworld=debug".into()),
        )
        .init();

    if let Err(e) = run().await {
        error!(error = %e, "helloworld: fatal");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    let cfg = ClientConfig::from_env().map_err(|e| anyhow::anyhow!("ClientConfig: {e}"))?;
    info!(endpoint = ?cfg.endpoint, "helloworld: connecting to broker");

    // ── DB bootstrap ──────────────────────────────────────────────────
    let pool = db::init_pool().await?;
    db::run_migrations(&pool).await?;
    info!("helloworld: db ready");

    // Late-bound BusClient (handlers need it for cross-app calls).
    let client_slot: Arc<OnceLock<Arc<BusClient>>> = Arc::new(OnceLock::new());
    let ctx = Arc::new(handlers::AppCtx {
        pool,
        client: Arc::clone(&client_slot),
    });

    // 数据面 socket 必须在 build_client 之前起来，才能把路径报给 broker。
    let data_plane_socket = data_plane::spawn("helloworld")
        .await
        .map_err(|e| anyhow::anyhow!("data_plane spawn: {e}"))?;

    let client = build_client(cfg, Arc::clone(&ctx), data_plane_socket)
        .await
        .map_err(|e| anyhow::anyhow!("bus build: {e}"))?;
    client_slot
        .set(Arc::clone(&client))
        .map_err(|_| anyhow::anyhow!("client_slot already set"))?;

    info!("helloworld: registered with broker");

    let shutdown = {
        let client = Arc::clone(&client);
        tokio::spawn(async move { client.run_until_shutdown().await })
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("helloworld: SIGINT received");
            client.shutdown();
        }
        _ = shutdown => info!("helloworld: broker sent Shutdown"),
    }

    Ok(())
}

async fn build_client(
    cfg: ClientConfig,
    ctx: Arc<handlers::AppCtx>,
    data_plane_socket: tokimo_bus_protocol::DataPlaneSocket,
) -> Result<Arc<BusClient>, tokimo_bus_protocol::BusError> {
    let ctx_list = Arc::clone(&ctx);
    let ctx_add = Arc::clone(&ctx);
    let ctx_del = Arc::clone(&ctx);
    let ctx_notify = Arc::clone(&ctx);

    BusClient::builder(cfg)
        .service("helloworld", env!("CARGO_PKG_VERSION"))
        .data_plane(data_plane_socket)
        .method(MethodDecl {
            name: "echo".into(),
            requires_auth: false,
            streaming: false,
            http_method: HttpMethod::Post,
            path: None,
            description: Some("Returns the request payload unchanged.".into()),
        })
        .method(MethodDecl {
            name: "greet".into(),
            requires_auth: false,
            streaming: false,
            http_method: HttpMethod::Post,
            path: None,
            description: Some("Returns a JSON greeting for `{ name }`.".into()),
        })
        .method(MethodDecl {
            name: "items.list".into(),
            requires_auth: false,
            streaming: false,
            http_method: HttpMethod::Post,
            path: None,
            description: Some("List recent items.".into()),
        })
        .method(MethodDecl {
            name: "items.add".into(),
            requires_auth: false,
            streaming: false,
            http_method: HttpMethod::Post,
            path: None,
            description: Some("Insert an item: { content }.".into()),
        })
        .method(MethodDecl {
            name: "items.delete".into(),
            requires_auth: false,
            streaming: false,
            http_method: HttpMethod::Post,
            path: None,
            description: Some("Delete an item: { id }.".into()),
        })
        .method(MethodDecl {
            name: "items.add_with_notify".into(),
            requires_auth: true,
            streaming: false,
            http_method: HttpMethod::Post,
            path: None,
            description: Some(
                "Insert an item, then emit a notification via notification_center.".into(),
            ),
        })
        .method(MethodDecl {
            name: "assets.get".into(),
            requires_auth: false,
            streaming: false,
            http_method: HttpMethod::Get,
            path: None,
            description: Some("Return embedded UI asset by relative path.".into()),
        })
        .on_invoke("echo", |req| async move { Ok(req.payload) })
        .on_invoke("greet", handlers::greet)
        .on_invoke("items.list", move |req| {
            let ctx = Arc::clone(&ctx_list);
            async move { handlers::items_list(ctx, req).await }
        })
        .on_invoke("items.add", move |req| {
            let ctx = Arc::clone(&ctx_add);
            async move { handlers::items_add(ctx, req).await }
        })
        .on_invoke("items.delete", move |req| {
            let ctx = Arc::clone(&ctx_del);
            async move { handlers::items_delete(ctx, req).await }
        })
        .on_invoke("items.add_with_notify", move |req| {
            let ctx = Arc::clone(&ctx_notify);
            async move { handlers::items_add_with_notify(ctx, req).await }
        })
        .on_invoke("assets.get", |req| async move {
            assets::handle(req.payload).await
        })
        .build()
        .await
}
