//! Axum handlers for helloworld。
//!
//! 共用 `AppCtx`：DB connection + 延迟绑定的 BusClient（用于 cross-app 调用）。

use std::sync::{Arc, OnceLock};

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use tokimo_bus_auth::TokimoUser;
use tokimo_bus_client::BusClient;
use tokimo_bus_protocol::CallerCtx;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    bus_clients::jobs,
    db::{entities::items, repos::items_repo::ItemsRepo},
};

pub struct AppCtx {
    pub db: DatabaseConnection,
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
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
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

impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        Self::internal(format!("db: {e}"))
    }
}

// ─── greet / echo ────────────────────────────────────────────────────────

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct GreetReq {
    name: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
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

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ItemDto {
    #[ts(type = "string")]
    pub id: Uuid,
    pub content: String,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

impl From<items::Model> for ItemDto {
    fn from(model: items::Model) -> Self {
        Self {
            id: model.id,
            content: model.content,
            created_at: model.created_at.with_timezone(&Utc),
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct ItemsListResp {
    items: Vec<ItemDto>,
}

pub async fn items_list(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<ItemsListResp>, AppError> {
    let user_id = parse_user_id(&user_id)?;
    let items = ItemsRepo::list_by_user(&ctx.db, user_id)
        .await?
        .into_iter()
        .map(ItemDto::from)
        .collect();
    Ok(Json(ItemsListResp { items }))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct AddReq {
    content: String,
}

pub async fn items_add(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<AddReq>,
) -> Result<Json<ItemDto>, AppError> {
    if req.content.trim().is_empty() {
        return Err(AppError::bad_request("content is empty"));
    }
    let user_id = parse_user_id(&user_id)?;
    let item = ItemsRepo::create(&ctx.db, user_id, req.content).await?;
    Ok(Json(ItemDto::from(item)))
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct DeleteResp {
    #[ts(type = "number")]
    deleted: u64,
}

pub async fn items_delete(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<DeleteResp>, AppError> {
    let user_id = parse_user_id(&user_id)?;
    let deleted = ItemsRepo::delete(&ctx.db, id, user_id).await?;
    Ok(Json(DeleteResp { deleted }))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct UpdateReq {
    content: String,
}

pub async fn items_update(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<UpdateReq>,
) -> Result<Json<ItemDto>, AppError> {
    if req.content.trim().is_empty() {
        return Err(AppError::bad_request("content is empty"));
    }

    let user_id = parse_user_id(&user_id)?;
    let item = ItemsRepo::update(&ctx.db, id, user_id, req.content)
        .await?
        .ok_or_else(|| AppError::not_found("item not found"))?;

    Ok(Json(ItemDto::from(item)))
}

pub async fn items_add_with_notify(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    headers: HeaderMap,
    Json(req): Json<AddReq>,
) -> Result<Json<ItemDto>, AppError> {
    if req.content.trim().is_empty() {
        return Err(AppError::bad_request("content is empty"));
    }

    let parsed_user_id = parse_user_id(&user_id)?;
    let item = ItemsRepo::create(&ctx.db, parsed_user_id, req.content).await?;
    let dto = ItemDto::from(item);

    let caller_user_id = Some(user_id);

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
        "user_id": caller_user_id,
        "app_id": "helloworld",
        "category_id": "item_added",
        "category_label": "helloworld.notifications.itemAdded",
        "title": "Helloworld",
        "body": format!("New item added: {}", dto.content),
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
                user_id: caller_user_id,
                caller_app_id: None,
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

// ─── jobs demo ────────────────────────────────────────────────────────────

const JOB_TYPE_BULK_IMPORT: &str = "helloworld_bulk_import";
const JOB_TYPE_LONG_RUNNING: &str = "helloworld_long_running";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartJobReq {
    #[serde(rename = "type")]
    job_type: String,
    #[serde(default = "empty_params")]
    params: JsonValue,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartJobResp {
    #[ts(type = "string")]
    job_id: Uuid,
}

pub async fn start_job(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<StartJobReq>,
) -> Result<Json<StartJobResp>, AppError> {
    let _parsed_user_id = parse_user_id(&user_id)?;
    if !matches!(req.job_type.as_str(), JOB_TYPE_BULK_IMPORT | JOB_TYPE_LONG_RUNNING) {
        return Err(AppError::bad_request("unsupported helloworld job type"));
    }

    let params = if req.params.is_null() { json!({}) } else { req.params };
    let client = bus_client(&ctx)?;
    let job = jobs::create(
        client.as_ref(),
        caller_for(&user_id),
        jobs::CreateJobRequest {
            job_type: req.job_type.clone(),
            params: params.clone(),
            data: None,
            parent_job_id: None,
            task_type: None,
            dedupe_key: None,
            priority: None,
        },
    )
    .await?;

    let job_id = job.id;
    let job_type = req.job_type;
    let task_user_id = user_id.clone();
    tokio::spawn(async move {
        if let Err(error) = run_simulated_job(client.clone(), task_user_id.clone(), job_id, job_type, params).await {
            let message = error.message;
            warn!(job_id = %job_id, error = %message, "helloworld: simulated job failed");
            if let Err(status_error) = jobs::update_status(
                client.as_ref(),
                caller_for(&task_user_id),
                jobs::UpdateStatusRequest {
                    job_id,
                    status: "failed".to_string(),
                    error: Some(message),
                    result: None,
                    progress: None,
                },
            )
            .await
            {
                warn!(job_id = %job_id, error = %status_error.message, "helloworld: failed to mark job failed");
            }
        }
    });

    Ok(Json(StartJobResp { job_id }))
}

async fn run_simulated_job(
    client: Arc<BusClient>,
    user_id: String,
    job_id: Uuid,
    job_type: String,
    params: JsonValue,
) -> Result<(), AppError> {
    jobs::update_status(
        client.as_ref(),
        caller_for(&user_id),
        jobs::UpdateStatusRequest {
            job_id,
            status: "running".to_string(),
            error: None,
            result: None,
            progress: Some(0),
        },
    )
    .await?;

    match job_type.as_str() {
        JOB_TYPE_BULK_IMPORT => run_bulk_import(client.as_ref(), &user_id, job_id, &params).await?,
        JOB_TYPE_LONG_RUNNING => run_long_running(client.as_ref(), &user_id, job_id, &params).await?,
        _ => return Err(AppError::bad_request("unsupported helloworld job type")),
    }

    jobs::update_status(
        client.as_ref(),
        caller_for(&user_id),
        jobs::UpdateStatusRequest {
            job_id,
            status: "completed".to_string(),
            error: None,
            result: None,
            progress: Some(100),
        },
    )
    .await?;

    Ok(())
}

async fn run_bulk_import(client: &BusClient, user_id: &str, job_id: Uuid, params: &JsonValue) -> Result<(), AppError> {
    let total = params
        .get("total")
        .or_else(|| params.get("count"))
        .and_then(JsonValue::as_u64)
        .unwrap_or(50)
        .max(1);

    for i in 0..total {
        sleep(Duration::from_millis(125)).await;
        let current = i + 1;
        jobs::update_progress(
            client,
            caller_for(user_id),
            job_id,
            ((current * 100) / total) as i32,
            Some(json!({ "progress": { "current": current, "total": total, "label": format!("Importing item #{}", current) } })),
        )
        .await?;
    }

    Ok(())
}

async fn run_long_running(client: &BusClient, user_id: &str, job_id: Uuid, params: &JsonValue) -> Result<(), AppError> {
    let duration_secs = params
        .get("duration_secs")
        .or_else(|| params.get("durationSecs"))
        .and_then(JsonValue::as_u64)
        .unwrap_or(30)
        .max(1);
    let total = duration_secs * 2;

    for i in 0..total {
        sleep(Duration::from_millis(500)).await;
        let current = i + 1;
        let elapsed_secs = current / 2;
        jobs::update_progress(
            client,
            caller_for(user_id),
            job_id,
            ((current * 100) / total) as i32,
            Some(json!({ "progress": { "current": current, "total": total, "label": format!("Working... {}s elapsed", elapsed_secs) } })),
        )
        .await?;
    }

    Ok(())
}

fn empty_params() -> JsonValue {
    json!({})
}

fn bus_client(ctx: &AppCtx) -> Result<Arc<BusClient>, AppError> {
    ctx.client
        .get()
        .cloned()
        .ok_or_else(|| AppError::internal("BusClient not yet bound"))
}

fn caller_for(user_id: &str) -> CallerCtx {
    CallerCtx {
        user_id: Some(user_id.to_string()),
        request_id: Uuid::new_v4().to_string(),
        workspace: None,
        caller_app_id: Some("helloworld".to_string()),
    }
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

fn parse_user_id(user_id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(user_id).map_err(|_| AppError::bad_request("invalid user id"))
}
