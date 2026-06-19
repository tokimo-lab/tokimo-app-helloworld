use chrono::Utc;
use sea_orm::{sea_query::Expr, *};
use serde_json::json;
use uuid::Uuid;

use crate::db::entities::{
    image_face_cache,
    person_faces::{self as pf, ActiveModel as PfActiveModel},
    person_media::{self as pm, ActiveModel as PmActiveModel},
    persons::{self, ActiveModel, Column, Entity},
};
use crate::error::AppError;

pub struct PersonRepo;

#[allow(dead_code)]
impl PersonRepo {
    pub async fn list<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<persons::Model>, AppError> {
        Ok(Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_desc(Column::UpdatedAt)
            .all(db)
            .await?)
    }

    pub async fn get_by_id<C: ConnectionTrait>(
        db: &C,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<persons::Model>, AppError> {
        Ok(Entity::find()
            .filter(Column::Id.eq(id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?)
    }

    pub async fn create<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        name: Option<String>,
    ) -> Result<persons::Model, AppError> {
        let now = Utc::now().fixed_offset();
        let am = ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            name: Set(name),
            avatar_url: Set(None),
            face_count: Set(0),
            metadata: Set(json!({})),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(am.insert(db).await?)
    }

    pub async fn update<C: ConnectionTrait>(
        db: &C,
        id: Uuid,
        user_id: Uuid,
        name: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<Option<persons::Model>, AppError> {
        let Some(model) = Self::get_by_id(db, id, user_id).await? else {
            return Ok(None);
        };
        let mut am: ActiveModel = model.into();
        if let Some(n) = name {
            am.name = Set(Some(n));
        }
        if let Some(url) = avatar_url {
            am.avatar_url = Set(Some(url));
        }
        am.updated_at = Set(Utc::now().fixed_offset());
        Ok(Some(am.update(db).await?))
    }

    pub async fn increment_face_count<C: ConnectionTrait>(db: &C, person_id: Uuid) -> Result<(), AppError> {
        Entity::update_many()
            .col_expr(Column::FaceCount, Expr::col(Column::FaceCount).add(1))
            .filter(Column::Id.eq(person_id))
            .exec(db)
            .await?;
        Ok(())
    }

    pub async fn decrement_face_count<C: ConnectionTrait>(db: &C, person_id: Uuid) -> Result<(), AppError> {
        Entity::update_many()
            .col_expr(Column::FaceCount, Expr::cust("GREATEST(face_count - 1, 0)"))
            .filter(Column::Id.eq(person_id))
            .exec(db)
            .await?;
        Ok(())
    }

    pub async fn delete_empty_persons<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<u64, AppError> {
        let result = Entity::delete_many()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::FaceCount.eq(0))
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }

    pub async fn link_face<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        person_id: Uuid,
        face_cache_id: Uuid,
    ) -> Result<pf::Model, AppError> {
        let am = PfActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            person_id: Set(person_id),
            face_cache_id: Set(face_cache_id),
            created_at: Set(Utc::now().fixed_offset()),
        };
        Ok(am.insert(db).await?)
    }

    pub async fn get_face_link<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        face_cache_id: Uuid,
    ) -> Result<Option<pf::Model>, AppError> {
        Ok(pf::Entity::find()
            .filter(pf::Column::UserId.eq(user_id))
            .filter(pf::Column::FaceCacheId.eq(face_cache_id))
            .one(db)
            .await?)
    }

    pub async fn get_person_faces<C: ConnectionTrait>(
        db: &C,
        person_id: Uuid,
    ) -> Result<Vec<(pf::Model, image_face_cache::Model)>, AppError> {
        Ok(pf::Entity::find()
            .filter(pf::Column::PersonId.eq(person_id))
            .find_also_related(image_face_cache::Entity)
            .all(db)
            .await?
            .into_iter()
            .filter_map(|(pf_model, cache)| cache.map(|c| (pf_model, c)))
            .collect())
    }

    pub async fn link_media<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        person_id: Uuid,
        source_app: &str,
        source_id: &str,
    ) -> Result<pm::Model, AppError> {
        let am = PmActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            person_id: Set(person_id),
            source_app: Set(source_app.to_string()),
            source_id: Set(source_id.to_string()),
            created_at: Set(Utc::now().fixed_offset()),
        };
        Ok(am.insert(db).await?)
    }

    pub async fn get_person_media<C: ConnectionTrait>(db: &C, person_id: Uuid) -> Result<Vec<pm::Model>, AppError> {
        Ok(pm::Entity::find()
            .filter(pm::Column::PersonId.eq(person_id))
            .order_by_desc(pm::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn delete_media_by_source<C: ConnectionTrait>(
        db: &C,
        source_app: &str,
        source_id: &str,
    ) -> Result<u64, AppError> {
        let result = pm::Entity::delete_many()
            .filter(pm::Column::SourceApp.eq(source_app))
            .filter(pm::Column::SourceId.eq(source_id))
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }

    pub async fn merge_persons<C: ConnectionTrait + TransactionTrait>(
        db: &C,
        user_id: Uuid,
        source_id: Uuid,
        target_id: Uuid,
    ) -> Result<(), AppError> {
        let txn = db.begin().await?;

        pf::Entity::update_many()
            .col_expr(pf::Column::PersonId, Expr::value(target_id))
            .filter(pf::Column::PersonId.eq(source_id))
            .exec(&txn)
            .await?;

        pm::Entity::update_many()
            .col_expr(pm::Column::PersonId, Expr::value(target_id))
            .filter(pm::Column::PersonId.eq(source_id))
            .exec(&txn)
            .await?;

        let source_faces = pf::Entity::find()
            .filter(pf::Column::PersonId.eq(source_id))
            .count(&txn)
            .await? as i32;

        Entity::update_many()
            .col_expr(Column::FaceCount, Expr::col(Column::FaceCount).add(source_faces))
            .filter(Column::Id.eq(target_id))
            .exec(&txn)
            .await?;

        Entity::delete_many()
            .filter(Column::Id.eq(source_id))
            .filter(Column::UserId.eq(user_id))
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }
}
