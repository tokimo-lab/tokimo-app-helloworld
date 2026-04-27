//! Axum handlers for helloworld。
//!
//! 共用 `AppCtx`：DB pool + 延迟绑定的 BusClient（用于 cross-app 调用）。

use std::sync::{Arc, OnceLock};

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokimo_bus_client::BusClient;
use tokimo_bus_protocol::CallerCtx;
use tracing::{info, warn};
use uuid::Uuid;

pub struct AppCtx {
    pub pool: PgPool,
    pub client: Arc<OnceLock<Arc<BusClient>>>,
}

/// 统一错误响应。
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.message });
        (self.status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::internal(format!("db: {e}"))
    }
}

// ─── greet / echo ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GreetReq {
    name: String,
}

#[derive(Serialize)]
pub struct GreetResp {
    message: String,
}

pub async fn greet(Json(req): Json<GreetReq>) -> Result<Json<GreetResp>, AppError> {
    Ok(Json(GreetResp {
        message: format!("Hello, {}!", req.name),
    }))
}

pub async fn echo(body: Bytes) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        body,
    )
        .into_response()
}

// ─── items CRUD ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ItemDto {
    pub id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ItemsListResp {
    items: Vec<ItemDto>,
}

pub async fn items_list(State(ctx): State<Arc<AppCtx>>) -> Result<Json<ItemsListResp>, AppError> {
    let rows = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        "SELECT id, content, created_at FROM items ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&ctx.pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|(id, content, created_at)| ItemDto {
            id,
            content,
            created_at,
        })
        .collect();
    Ok(Json(ItemsListResp { items }))
}

#[derive(Deserialize)]
pub struct AddReq {
    content: String,
}

pub async fn items_add(State(ctx): State<Arc<AppCtx>>, Json(req): Json<AddReq>) -> Result<Json<ItemDto>, AppError> {
    if req.content.trim().is_empty() {
        return Err(AppError::bad_request("content is empty"));
    }
    let row = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        "INSERT INTO items(content) VALUES ($1) RETURNING id, content, created_at",
    )
    .bind(&req.content)
    .fetch_one(&ctx.pool)
    .await?;
    Ok(Json(ItemDto {
        id: row.0,
        content: row.1,
        created_at: row.2,
    }))
}

#[derive(Serialize)]
pub struct DeleteResp {
    deleted: u64,
}

pub async fn items_delete(State(ctx): State<Arc<AppCtx>>, Path(id): Path<Uuid>) -> Result<Json<DeleteResp>, AppError> {
    let res = sqlx::query("DELETE FROM items WHERE id = $1")
        .bind(id)
        .execute(&ctx.pool)
        .await?;
    Ok(Json(DeleteResp {
        deleted: res.rows_affected(),
    }))
}

pub async fn items_add_with_notify(
    State(ctx): State<Arc<AppCtx>>,
    headers: HeaderMap,
    Json(req): Json<AddReq>,
) -> Result<Json<ItemDto>, AppError> {
    if req.content.trim().is_empty() {
        return Err(AppError::bad_request("content is empty"));
    }

    let row = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
        "INSERT INTO items(content) VALUES ($1) RETURNING id, content, created_at",
    )
    .bind(&req.content)
    .fetch_one(&ctx.pool)
    .await?;
    let dto = ItemDto {
        id: row.0,
        content: row.1.clone(),
        created_at: row.2,
    };

    // 从 server 注入的 header 提取 user_id（方案 3 的标准做法）
    let user_id = headers
        .get("x-tokimo-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let client = ctx
        .client
        .get()
        .ok_or_else(|| AppError::internal("BusClient not yet bound"))?;

    let notify_payload = serde_json::json!({
        "user_id": user_id,
        "app_id": "helloworld",
        "category_id": "item_added",
        "category_label": "helloworld.notifications.itemAdded",
        "title": "Helloworld",
        "body": format!("New item added: {}", row.1),
        "level": "info",
    });
    let bytes =
        serde_json::to_vec(&notify_payload).map_err(|e| AppError::internal(format!("serialize notify: {e}")))?;

    info!(item_id = %dto.id, "helloworld: dispatching notification_center.notify");
    if let Err(e) = client
        .invoke(
            "notification_center",
            "notify",
            bytes,
            CallerCtx {
                user_id,
                request_id,
                workspace: None,
            },
        )
        .await
    {
        warn!(error = %e, "notification dispatch failed (item still saved)");
    }

    Ok(Json(dto))
}

// ─── data plane 示例 ─────────────────────────────────────────────────────

pub async fn data_hello() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "hello from helloworld data-plane\n",
    )
        .into_response()
}
