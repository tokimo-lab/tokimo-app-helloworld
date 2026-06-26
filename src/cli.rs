//! CLI entrypoints for helloworld。

use anyhow::Context;
use chrono::Utc;
use tokimo_bus_auth::db::verify_token;
use tokimo_bus_cli::{Credentials, TokimoAuthArgs};
use uuid::Uuid;

use crate::{
    ItemsCmd,
    db::{init_pool, repos::items_repo::ItemsRepo},
};

pub async fn run_items(auth: TokimoAuthArgs, cmd: ItemsCmd) -> anyhow::Result<()> {
    let (db, user_id) = init(auth).await?;

    match cmd {
        ItemsCmd::List => {
            let items = ItemsRepo::list_by_user(&db, user_id)
                .await
                .context("list items failed")?;
            if items.is_empty() {
                println!("No items.");
                return Ok(());
            }

            println!("{:<36}  {:<25}  Content", "ID", "Created At");
            for item in items {
                println!(
                    "{:<36}  {:<25}  {}",
                    item.id,
                    item.created_at.with_timezone(&Utc).to_rfc3339(),
                    item.content
                );
            }
        }
        ItemsCmd::Add { content } => {
            let item = ItemsRepo::create(&db, user_id, content)
                .await
                .context("add item failed")?;
            println!("Added item {}: {}", item.id, item.content);
        }
        ItemsCmd::Update { id, content } => {
            let item = ItemsRepo::update(&db, id, user_id, content)
                .await
                .context("update item failed")?
                .ok_or_else(|| anyhow::anyhow!("item not found"))?;
            println!("Updated item {}: {}", item.id, item.content);
        }
        ItemsCmd::Delete { id } => {
            let rows = ItemsRepo::delete(&db, id, user_id)
                .await
                .context("delete item failed")?;
            if rows == 0 {
                anyhow::bail!("item not found");
            }
            println!("Deleted item {id}");
        }
    }

    Ok(())
}

pub async fn run_greet(auth: TokimoAuthArgs, name: String) -> anyhow::Result<()> {
    let _ = init(auth).await?;
    println!("Hello, {name}!");
    Ok(())
}

async fn init(auth: TokimoAuthArgs) -> anyhow::Result<(sea_orm::DatabaseConnection, Uuid)> {
    let credentials = Credentials::resolve(&auth).context("resolve Tokimo credentials failed")?;
    let db = init_pool().await.context("connect database failed")?;
    let verified = verify_token(&db, &credentials.token)
        .await
        .context("verify Tokimo token failed")?;
    Ok((db, verified.user_id))
}
