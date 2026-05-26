use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokimo_bus_client::BusClient;
use tokimo_bus_protocol::CallerCtx;
use uuid::Uuid;

use crate::handlers::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub job_type: String,
    pub status: String,
    pub user_id: Option<Uuid>,
    pub parent_job_id: Option<Uuid>,
    pub task_type: Option<String>,
    pub params: JsonValue,
    pub data: Option<JsonValue>,
    pub progress: i32,
    pub priority: i32,
    pub error: Option<String>,
    pub started_at: Option<DateTime<FixedOffset>>,
    pub completed_at: Option<DateTime<FixedOffset>>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    #[serde(rename = "kind")]
    pub job_type: String,
    pub params: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusRequest {
    pub job_id: Uuid,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgressRequest {
    job_id: Uuid,
    progress: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<JsonValue>,
}

pub async fn create(client: &BusClient, caller: CallerCtx, request: CreateJobRequest) -> Result<JobView, AppError> {
    let response = invoke_json(client, "create", caller, &request).await?;
    serde_json::from_slice::<JobView>(&response)
        .map_err(|error| AppError::internal(format!("jobs.create decode: {error}")))
}

pub async fn update_status(
    client: &BusClient,
    caller: CallerCtx,
    request: UpdateStatusRequest,
) -> Result<JobView, AppError> {
    let response = invoke_json(client, "update_status", caller, &request).await?;
    serde_json::from_slice::<JobView>(&response)
        .map_err(|error| AppError::internal(format!("jobs.update_status decode: {error}")))
}

pub async fn update_progress(
    client: &BusClient,
    caller: CallerCtx,
    job_id: Uuid,
    progress: i32,
    progress_data: Option<JsonValue>,
) -> Result<JobView, AppError> {
    let req = UpdateProgressRequest {
        job_id,
        progress,
        data: progress_data,
    };
    let response = invoke_json(client, "update_progress", caller, &req).await?;
    serde_json::from_slice::<JobView>(&response)
        .map_err(|error| AppError::internal(format!("jobs.update_progress decode: {error}")))
}

async fn invoke_json<T: Serialize>(
    client: &BusClient,
    method: &str,
    caller: CallerCtx,
    request: &T,
) -> Result<Vec<u8>, AppError> {
    let payload =
        serde_json::to_vec(request).map_err(|error| AppError::internal(format!("jobs.{method} encode: {error}")))?;
    client
        .invoke("jobs", method, payload, caller)
        .await
        .map_err(|error| AppError::internal(format!("jobs.{method} via bus: {error}")))
}
