use std::sync::Arc;

use axum::{Router, middleware, routing::get};
use tokimo_bus_protocol::{BusListener, DataPlaneSocket};
use tracing::{error, info};

use crate::{assets, handlers, state::AppState};

pub async fn spawn(service: &str, ctx: Arc<AppState>) -> anyhow::Result<DataPlaneSocket> {
    let (listener, socket) = BusListener::bind_for_app(service)?;
    info!(?socket, "person: app server listening");

    let router = build_router(ctx);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            error!(error = %e, "person: app server stopped");
        }
    });

    Ok(socket)
}

fn build_router(ctx: Arc<AppState>) -> Router {
    Router::new()
        .route("/persons", get(handlers::list_persons))
        .route("/persons/{id}", get(handlers::get_person).put(handlers::update_person))
        .route("/assets/{*path}", get(assets::serve))
        .layer(middleware::from_fn(tokimo_bus_protocol::task_local::auth_middleware))
        .with_state(ctx)
}
