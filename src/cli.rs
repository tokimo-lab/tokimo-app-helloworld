//! CLI entrypoints for helloworld。

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokimo_bus_auth::cli::{Client, Credentials, TokimoAuthArgs};
use uuid::Uuid;

use crate::ItemsCmd;

#[derive(Deserialize)]
struct ItemDto {
    id: Uuid,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct ItemsListResp {
    items: Vec<ItemDto>,
}

#[derive(Serialize)]
struct ItemContentReq {
    content: String,
}

#[derive(Serialize)]
struct GreetReq {
    name: String,
}

#[derive(Deserialize)]
struct GreetResp {
    message: String,
}

pub async fn run_items(auth: TokimoAuthArgs, cmd: ItemsCmd) -> anyhow::Result<()> {
    let client = client(auth)?;

    match cmd {
        ItemsCmd::List => {
            let resp: ItemsListResp = client.get("/items").await.context("list items failed")?;
            if resp.items.is_empty() {
                println!("No items.");
                return Ok(());
            }

            println!("{:<36}  {:<25}  Content", "ID", "Created At");
            for item in resp.items {
                println!(
                    "{:<36}  {:<25}  {}",
                    item.id,
                    item.created_at.to_rfc3339(),
                    item.content
                );
            }
        }
        ItemsCmd::Add { content } => {
            let req = ItemContentReq { content };
            let item: ItemDto = client.post("/items", &req).await.context("add item failed")?;
            println!("Added item {}: {}", item.id, item.content);
        }
        ItemsCmd::Update { id, content } => {
            let req = ItemContentReq { content };
            let item: ItemDto = client
                .put(&format!("/items/{id}"), &req)
                .await
                .context("update item failed")?;
            println!("Updated item {}: {}", item.id, item.content);
        }
        ItemsCmd::Delete { id } => {
            client
                .delete(&format!("/items/{id}"))
                .await
                .context("delete item failed")?;
            println!("Deleted item {id}");
        }
    }

    Ok(())
}

pub async fn run_greet(auth: TokimoAuthArgs, name: String) -> anyhow::Result<()> {
    let client = client(auth)?;
    let resp: GreetResp = client
        .post("/greet", &GreetReq { name })
        .await
        .context("greet failed")?;

    println!("{}", resp.message);
    Ok(())
}

fn client(auth: TokimoAuthArgs) -> anyhow::Result<Client> {
    let credentials = Credentials::resolve(&auth).context("resolve Tokimo credentials failed")?;
    Ok(Client::new("helloworld", credentials))
}
