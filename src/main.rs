//! Helloworld app — 方案 3 形态：内嵌 axum + UDS。
//!
//! 启动流程：
//! 1. 连接 broker（仅用于 supervisor 健康检查 + 可选的 cross-app `notification_center.notify`）
//! 2. 起 axum router 监听 `<runtime_dir>/apps/helloworld.sock`
//! 3. 把这个 sock 报给 broker（沿用 `data_plane_socket` 字段）
//! 4. server 端的 `/api/apps/helloworld/<rest>` 全部反代到这个 sock 的 `/<rest>`
//!
//! 与旧版的差别：
//! - 不再调用 `BusClient::builder().method(...).on_invoke(...)`
//! - 业务路由改成标准 axum handler signature
//! - 数据流 / 静态资源 / 业务方法 共用同一个 sock（同一个 axum router）

mod app_server;
mod assets;
mod db;
mod handlers;

use std::sync::{Arc, OnceLock};

use tokimo_bus_client::{BusClient, ClientConfig};
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

    let pool = db::init_pool().await?;
    db::run_migrations(&pool).await?;
    info!("helloworld: db ready");

    // BusClient 仍然存在 —— 不为暴露方法，而是：
    // 1) 让 broker 知道 helloworld 在线（supervisor 健康检查）
    // 2) 提供 cross-app `bus.call("notification_center", "notify", ...)` 通道
    let client_slot: Arc<OnceLock<Arc<BusClient>>> = Arc::new(OnceLock::new());
    let ctx = Arc::new(handlers::AppCtx {
        pool,
        client: Arc::clone(&client_slot),
    });

    // 起 axum router 监听 UDS（业务 + assets + data 都在这个 sock 上）
    let app_socket = app_server::spawn("helloworld", Arc::clone(&ctx))
        .await
        .map_err(|e| anyhow::anyhow!("app_server spawn: {e}"))?;

    // 把 sock 通过 `data_plane_socket` 上报给 broker（server 用它做反代目的地）
    let client = BusClient::builder(cfg)
        .service("helloworld", env!("CARGO_PKG_VERSION"))
        .data_plane(app_socket)
        .build()
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
