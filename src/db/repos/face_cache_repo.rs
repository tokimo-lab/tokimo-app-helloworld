use chrono::Utc;
use sea_orm::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::entities::image_face_cache::{self, ActiveModel, Column, Entity};
use crate::error::AppError;

pub struct FaceCacheRepo;

#[allow(dead_code)]
impl FaceCacheRepo {
    pub async fn get_by_image_hash<C: ConnectionTrait>(
        db: &C,
        image_hash: &str,
    ) -> Result<Vec<image_face_cache::Model>, AppError> {
        Ok(Entity::find()
            .filter(Column::ImageHash.eq(image_hash))
            .order_by_asc(Column::FaceIndex)
            .all(db)
            .await?)
    }

    pub async fn get_by_id<C: ConnectionTrait>(db: &C, id: Uuid) -> Result<Option<image_face_cache::Model>, AppError> {
        Ok(Entity::find_by_id(id).one(db).await?)
    }

    pub async fn upsert_faces<C: ConnectionTrait>(
        db: &C,
        image_hash: &str,
        source_app: &str,
        source_id: &str,
        faces: &[JsonValue],
    ) -> Result<Vec<image_face_cache::Model>, AppError> {
        let existing = Self::get_by_image_hash(db, image_hash).await?;
        if !existing.is_empty() {
            return Ok(existing);
        }

        let now = Utc::now().fixed_offset();
        let models: Vec<ActiveModel> = faces
            .iter()
            .enumerate()
            .map(|(i, face)| ActiveModel {
                id: Set(Uuid::new_v4()),
                image_hash: Set(image_hash.to_string()),
                source_app: Set(source_app.to_string()),
                source_id: Set(source_id.to_string()),
                face_index: Set(i as i32),
                bbox: Set(face.clone()),
                created_at: Set(now),
            })
            .collect();

        Entity::insert_many(models)
            .on_conflict(
                sea_query::OnConflict::columns([Column::ImageHash, Column::FaceIndex])
                    .do_nothing()
                    .to_owned(),
            )
            .exec(db)
            .await?;

        Self::get_by_image_hash(db, image_hash).await
    }

    pub async fn delete_by_source<C: ConnectionTrait>(
        db: &C,
        source_app: &str,
        source_id: &str,
    ) -> Result<u64, AppError> {
        let result = Entity::delete_many()
            .filter(Column::SourceApp.eq(source_app))
            .filter(Column::SourceId.eq(source_id))
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }
}
