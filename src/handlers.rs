//! Bus method handlers for helloworld.

use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokimo_bus_client::{BusClient, InvokeRequest};
use tokimo_bus_protocol::{BusError, CallerCtx};
use tracing::{info, warn};
use uuid::Uuid;

pub struct AppCtx {
    pub pool: PgPool,
    /// Late-bound — set by `main` after `BusClient` is built.
    pub client: Arc<OnceLock<Arc<BusClient>>>,
}

#[derive(Deserialize)]
struct GreetReq {
    name: String,
}
#[derive(Serialize)]
struct GreetResp {
    message: String,
}

pub async fn greet(req: InvokeRequest) -> Result<Vec<u8>, BusError> {
    let GreetReq { name } = serde_json::from_slice(&req.payload)
        .map_err(|e| BusError::BadRequest(format!("greet: {e}")))?;
    let resp = GreetResp {
        message: format!("Hello, {name}!"),
    };
    serde_json::to_vec(&resp).map_err(|e| BusError::Internal(e.to_string()))
}

#[derive(Serialize)]
pub struct ItemDto {
    pub id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct ItemsListResp {
    items: Vec<ItemDto>,
}

pub async fn items_list(ctx: Arc<AppCtx>, _req: InvokeRequest) -> Result<Vec<u8>, BusError> {
    let rows = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        "SELECT id, content, created_at FROM items ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&ctx.pool)
    .await
    .map_err(db_err)?;

    let items = rows
        .into_iter()
        .map(|(id, content, created_at)| ItemDto {
            id,
            content,
            created_at,
        })
        .collect::<Vec<_>>();

    serde_json::to_vec(&ItemsListResp { items }).map_err(|e| BusError::Internal(e.to_string()))
}

#[derive(Deserialize)]
struct AddReq {
    content: String,
}

pub async fn items_add(ctx: Arc<AppCtx>, req: InvokeRequest) -> Result<Vec<u8>, BusError> {
    let AddReq { content } = serde_json::from_slice(&req.payload)
        .map_err(|e| BusError::BadRequest(format!("items.add: {e}")))?;
    if content.trim().is_empty() {
        return Err(BusError::BadRequest("content is empty".into()));
    }
    let row = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        "INSERT INTO items(content) VALUES ($1) RETURNING id, content, created_at",
    )
    .bind(&content)
    .fetch_one(&ctx.pool)
    .await
    .map_err(db_err)?;

    let dto = ItemDto {
        id: row.0,
        content: row.1,
        created_at: row.2,
    };
    serde_json::to_vec(&dto).map_err(|e| BusError::Internal(e.to_string()))
}

#[derive(Deserialize)]
struct DeleteReq {
    id: Uuid,
}
#[derive(Serialize)]
struct DeleteResp {
    deleted: u64,
}

pub async fn items_delete(ctx: Arc<AppCtx>, req: InvokeRequest) -> Result<Vec<u8>, BusError> {
    let DeleteReq { id } = serde_json::from_slice(&req.payload)
        .map_err(|e| BusError::BadRequest(format!("items.delete: {e}")))?;
    let res = sqlx::query("DELETE FROM items WHERE id = $1")
        .bind(id)
        .execute(&ctx.pool)
        .await
        .map_err(db_err)?;
    serde_json::to_vec(&DeleteResp {
        deleted: res.rows_affected(),
    })
    .map_err(|e| BusError::Internal(e.to_string()))
}

pub async fn items_add_with_notify(
    ctx: Arc<AppCtx>,
    req: InvokeRequest,
) -> Result<Vec<u8>, BusError> {
    let AddReq { content } = serde_json::from_slice(&req.payload)
        .map_err(|e| BusError::BadRequest(format!("items.add_with_notify: {e}")))?;
    if content.trim().is_empty() {
        return Err(BusError::BadRequest("content is empty".into()));
    }

    let row = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        "INSERT INTO items(content) VALUES ($1) RETURNING id, content, created_at",
    )
    .bind(&content)
    .fetch_one(&ctx.pool)
    .await
    .map_err(db_err)?;
    let dto = ItemDto {
        id: row.0,
        content: row.1.clone(),
        created_at: row.2,
    };

    let client = ctx
        .client
        .get()
        .ok_or_else(|| BusError::Internal("BusClient not yet bound".into()))?;

    // Forward caller identity so notification_center can target the right user.
    let notify_payload = serde_json::json!({
        "user_id": req.caller.user_id,
        "app_id": "helloworld",
        "category_id": "item_added",
        "category_label": "helloworld.notifications.itemAdded",
        "title": "Helloworld",
        "body": format!("New item added: {}", row.1),
        "level": "info",
    });
    let bytes = serde_json::to_vec(&notify_payload).map_err(|e| BusError::Internal(e.to_string()))?;

    info!(item_id = %dto.id, "helloworld: dispatching notification_center.notify");
    if let Err(e) = client
        .invoke(
            "notification_center",
            "notify",
            bytes,
            CallerCtx {
                user_id: req.caller.user_id.clone(),
                request_id: req.caller.request_id.clone(),
                workspace: req.caller.workspace.clone(),
            },
        )
        .await
    {
        warn!(error = %e, "notification dispatch failed (item still saved)");
    }

    serde_json::to_vec(&dto).map_err(|e| BusError::Internal(e.to_string()))
}

fn db_err(e: sqlx::Error) -> BusError {
    BusError::Internal(format!("db: {e}"))
}
