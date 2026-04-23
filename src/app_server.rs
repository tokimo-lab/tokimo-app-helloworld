//! 内嵌 axum HTTP server，监听本地 UDS。
//!
//! 路由布局（server 端 `/api/apps/helloworld/<rest>` 反代到本 sock 的 `/<rest>`）：
//! - `GET    /items`                   → 列表
//! - `POST   /items`                   → 新增
//! - `DELETE /items/{id}`              → 删除
//! - `POST   /items/notify`            → 新增并触发通知
//! - `POST   /greet`                   → 演示 typed JSON
//! - `POST   /echo`                    → 透传 body
//! - `GET    /assets/{*path}`          → 静态资源
//! - `GET    /data/hello.txt`          → 数据流示例
//!
//! 单 sock 同时承载控制面 + 数据面 + 资源面，server 侧只需一条反代规则。

use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    routing::{any, delete, get, post},
};
use tokimo_bus_protocol::DataPlaneSocket;
use tokio::net::UnixListener;
use tracing::{error, info};

use crate::{assets, handlers, handlers::AppCtx};

/// 根据 broker socket 路径推出 app 自己的 sock 路径。
fn default_socket_path(service: &str) -> anyhow::Result<PathBuf> {
    let bus = std::env::var("TOKIMO_BUS_SOCKET")
        .map_err(|_| anyhow::anyhow!("TOKIMO_BUS_SOCKET not set"))?;
    let parent = PathBuf::from(&bus)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("TOKIMO_BUS_SOCKET has no parent"))?
        .to_path_buf();
    let apps_dir = parent.join("apps");
    std::fs::create_dir_all(&apps_dir)?;
    Ok(apps_dir.join(format!("{service}.sock")))
}

/// 起 axum server 监听 UDS，返回 `DataPlaneSocket` 用于上报 broker。
pub async fn spawn(service: &str, ctx: Arc<AppCtx>) -> anyhow::Result<DataPlaneSocket> {
    let path = default_socket_path(service)?;
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    info!(path = %path.display(), "helloworld: app server listening");

    let router = build_router(ctx);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            error!(error = %e, "helloworld: app server stopped");
        }
    });

    Ok(DataPlaneSocket::Unix {
        path: path.to_string_lossy().into_owned(),
    })
}

fn build_router(ctx: Arc<AppCtx>) -> Router {
    Router::new()
        .route("/items", get(handlers::items_list).post(handlers::items_add))
        .route("/items/{id}", delete(handlers::items_delete))
        .route("/items/notify", post(handlers::items_add_with_notify))
        .route("/greet", post(handlers::greet))
        .route("/echo", any(handlers::echo))
        .route("/assets/{*path}", get(assets::serve))
        .route("/data/hello.txt", get(handlers::data_hello))
        .with_state(ctx)
}
