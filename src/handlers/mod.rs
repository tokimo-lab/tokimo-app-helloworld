use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokimo_bus_auth::TokimoUser;
use ts_rs::TS;
use uuid::Uuid;

use crate::{db::repos::person_repo::PersonRepo, error::AppError};

pub type AppCtx = crate::state::AppState;

fn parse_user_id(user_id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(user_id).map_err(|_| AppError::BadRequest("invalid user id".into()))
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PersonDto {
    #[ts(type = "string")]
    pub id: Uuid,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub face_count: i32,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub updated_at: DateTime<Utc>,
}

impl From<crate::db::entities::persons::Model> for PersonDto {
    fn from(m: crate::db::entities::persons::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            avatar_url: m.avatar_url,
            face_count: m.face_count,
            created_at: m.created_at.with_timezone(&Utc),
            updated_at: m.updated_at.with_timezone(&Utc),
        }
    }
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct PersonListResponse {
    persons: Vec<PersonDto>,
}

pub async fn list_persons(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<PersonListResponse>, AppError> {
    let uid = parse_user_id(&user_id)?;
    let persons = PersonRepo::list(&ctx.db, uid)
        .await?
        .into_iter()
        .map(PersonDto::from)
        .collect();
    Ok(Json(PersonListResponse { persons }))
}

pub async fn get_person(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<PersonDto>, AppError> {
    let uid = parse_user_id(&user_id)?;
    let person = PersonRepo::get_by_id(&ctx.db, id, uid)
        .await?
        .ok_or_else(|| AppError::NotFound("person not found".into()))?;
    Ok(Json(PersonDto::from(person)))
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct UpdatePersonReq {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

pub async fn update_person(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<UpdatePersonReq>,
) -> Result<Json<PersonDto>, AppError> {
    let uid = parse_user_id(&user_id)?;
    let person = PersonRepo::update(&ctx.db, id, uid, req.name, req.avatar_url)
        .await?
        .ok_or_else(|| AppError::NotFound("person not found".into()))?;
    Ok(Json(PersonDto::from(person)))
}
