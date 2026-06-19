use anyhow::Context;
use uuid::Uuid;

use crate::db::{init_pool, repos::person_repo::PersonRepo};

pub async fn run_list(user_id: String) -> anyhow::Result<()> {
    let db = init_pool().await.context("connect database failed")?;
    let uid = Uuid::parse_str(&user_id).context("invalid user_id")?;

    let persons = PersonRepo::list(&db, uid).await.context("list persons failed")?;

    if persons.is_empty() {
        println!("No persons.");
        return Ok(());
    }

    println!("{:<36}  {:<6}  Name", "ID", "Faces");
    for p in persons {
        println!(
            "{:<36}  {:<6}  {}",
            p.id,
            p.face_count,
            p.name.as_deref().unwrap_or("(unnamed)")
        );
    }
    Ok(())
}

pub async fn run_match_face(user_id: String, image_hash: String, face_index: i32) -> anyhow::Result<()> {
    let db = init_pool().await.context("connect database failed")?;
    let uid = Uuid::parse_str(&user_id).context("invalid user_id")?;

    let faces = crate::db::repos::face_cache_repo::FaceCacheRepo::get_by_image_hash(&db, &image_hash)
        .await
        .context("get face cache failed")?;

    let face = faces
        .into_iter()
        .find(|f| f.face_index == face_index)
        .ok_or_else(|| anyhow::anyhow!("face index {face_index} not found for image {image_hash}"))?;

    let link = PersonRepo::get_face_link(&db, uid, face.id)
        .await
        .context("get face link failed")?;

    match link {
        Some(l) => {
            println!("Matched person: {}", l.person_id);
            println!("Face cache ID: {}", face.id);
        }
        None => {
            println!("No person match for face {}", face.id);
            println!("Face cache ID: {}", face.id);
        }
    }
    Ok(())
}
