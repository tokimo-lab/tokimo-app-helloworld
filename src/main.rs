//! Person app — 中心化人物身份服务。
//!
//! 职责：
//! - 共享层：image_face_cache 存储人脸检测结果（embedding + bbox），按图片 hash 去重
//! - 用户层：persons / person_faces / person_media 管理用户的人物分组与关联
//! - Bus API：提供人脸匹配、人物 CRUD、跨 app 关联查询
//!
//! 启动流程：
//! 1. 连接 broker
//! 2. 初始化 DB（schema 由 host 管理）
//! 3. 注册 bus services（person.match_face 等）
//! 4. 起 axum router 监听 UDS
//! 5. 把 sock 上报给 broker

const MANIFEST: &str = include_str!("../tokimo-app.toml");

mod app_server;
mod assets;
mod bus_clients;
mod bus_services;
mod cli;
mod db;
mod error;
mod handlers;
mod state;

use std::sync::{Arc, OnceLock};

use clap::{Parser, Subcommand};
use tokimo_bus_cli::TokimoAuthArgs;
use tokimo_bus_client::{BusClient, ClientConfig};
use tracing::{error, info};

use crate::state::AppState;

#[derive(Parser, Debug)]
#[command(
    name = "tokimo-app-person",
    about = "Person — 中心化人物身份服务",
    long_about = "Person CLI — 管理人物身份、人脸匹配、跨 app 关联。",
    term_width = 100
)]
struct Cli {
    #[command(flatten)]
    auth: TokimoAuthArgs,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List persons for a user
    List {
        /// User ID (UUID)
        #[arg(short, long)]
        user_id: String,
    },
    /// Match a face embedding against known persons
    MatchFace {
        /// User ID (UUID)
        #[arg(short, long)]
        user_id: String,
        /// Image hash
        #[arg(long)]
        image_hash: String,
        /// Face index in the image
        #[arg(long, default_value = "0")]
        face_index: i32,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { auth: _, command } = Cli::parse();

    match command {
        None if std::env::var_os("TOKIMO_BUS_SOCKET").is_some() => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info,tokimo_bus_client=info,tokimo_app_person=debug".into()),
                )
                .init();
            if let Err(error) = run_server().await {
                error!(%error, "person: fatal");
                std::process::exit(1);
            }
        }
        None => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            tokimo_bus_cli::print_help_unified(&mut cmd);
            std::process::exit(0);
        }
        Some(cmd) => match cmd {
            Command::List { user_id } => {
                cli::run_list(user_id).await?;
            }
            Command::MatchFace {
                user_id,
                image_hash,
                face_index,
            } => {
                cli::run_match_face(user_id, image_hash, face_index).await?;
            }
        },
    }

    Ok(())
}

async fn run_server() -> anyhow::Result<()> {
    let cfg = ClientConfig::from_env().map_err(|e| anyhow::anyhow!("ClientConfig: {e}"))?;
    info!(endpoint = ?cfg.endpoint, "person: connecting to broker");

    let db = db::init_pool().await?;
    info!("person: db connected (schema managed by host)");

    let client_slot: Arc<OnceLock<Arc<BusClient>>> = Arc::new(OnceLock::new());

    let ctx = Arc::new(AppState {
        db,
        bus_client: Arc::clone(&client_slot),
    });

    let app_socket = app_server::spawn("person", Arc::clone(&ctx))
        .await
        .map_err(|e| anyhow::anyhow!("app_server spawn: {e}"))?;

    let builder = BusClient::builder(cfg)
        .service("person", env!("CARGO_PKG_VERSION"))
        .data_plane(app_socket);

    let builder = bus_services::person::register(builder, Arc::clone(&ctx));

    let client = builder.build().await.map_err(|e| anyhow::anyhow!("bus build: {e}"))?;

    client_slot
        .set(Arc::clone(&client))
        .map_err(|_| anyhow::anyhow!("client_slot already set"))?;

    info!("person: registered with broker");

    let shutdown = {
        let client = Arc::clone(&client);
        tokio::spawn(async move { client.run_until_shutdown().await })
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("person: SIGINT received");
            client.shutdown();
        }
        _ = shutdown => info!("person: broker sent Shutdown"),
    }

    Ok(())
}
