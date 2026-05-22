//! Helloworld app — 方案 3 形态：内嵌 axum + UDS。
//!
//! 启动流程：
//! 1. 连接 broker（仅用于 supervisor 健康检查 + 可选的 cross-app `notification_center.notify`）
//! 2. 起 axum router 监听 `<runtime_dir>/apps/helloworld.sock`
//! 3. 把这个 sock 报给 broker（沿用 `data_plane_socket` 字段）
//! 4. server 端的 `/api/apps/helloworld/<rest>` 全部反代到这个 sock 的 `/<rest>`
//!
//! 与旧版的差别：
//! - 不再调用 `BusClient::builder().method(...).on_invoke(...)`
//! - 业务路由改成标准 axum handler signature
//! - 数据流 / 静态资源 / 业务方法 共用同一个 sock（同一个 axum router）

/// Compile-time embedded app manifest; shared with the library crate via lib.rs.
const MANIFEST: &str = include_str!("../tokimo-app.toml");

mod app_server;
mod assets;
mod cli;
mod db;
mod handlers;

use std::sync::{Arc, OnceLock};

use clap::{Parser, Subcommand};
use tokimo_bus_cli::TokimoAuthArgs;
use tokimo_bus_client::{BusClient, ClientConfig};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(
    name = "tokimo-app-helloworld",
    about = "Helloworld — Tokimo 子 app CLI",
    long_about = "Helloworld CLI — 直接读写 Tokimo 数据库，管理 helloworld items。\n\n前置条件：\n1. 在浏览器登录 Tokimo 后，去「设置 → API Keys」创建一个 token (mm_xxx)\n2. 把 token 通过 --tokimo-token 或 TOKIMO_TOKEN env 传入\n3. 确保 DATABASE_URL env 指向 Tokimo 数据库（与主 server 一致）\n\nCLI 直接读写数据库，不依赖主 server 进程运行。",
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
    /// 管理 helloworld items
    #[command(
        subcommand_required = false,
        arg_required_else_help = false,
        long_about = "管理 helloworld items",
        term_width = 100
    )]
    Items {
        #[command(subcommand)]
        cmd: Option<ItemsCmd>,
    },
    /// 打印问候语
    Greet { name: String },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ItemsCmd {
    /// 列出最近 100 条 item
    List,
    /// 新增一条 item
    Add {
        /// item 内容（非空字符串）
        content: String,
    },
    /// 更新 item 内容
    Update {
        /// item ID (UUID)
        id: uuid::Uuid,
        /// 新内容
        content: String,
    },
    /// 删除指定 item
    Delete {
        /// item ID (UUID)
        id: uuid::Uuid,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { auth, command } = Cli::parse();

    match command {
        None if std::env::var_os("TOKIMO_BUS_SOCKET").is_some() => {
            // server 模式：由 supervisor 无参拉起（注入了 TOKIMO_BUS_SOCKET），初始化 tracing
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info,tokimo_bus_client=info,tokimo_app_helloworld=debug".into()),
                )
                .init();
            if let Err(error) = run_server().await {
                error!(%error, "helloworld: fatal");
                std::process::exit(1);
            }
        }
        None => {
            // 人手动无参运行：打印 CLI help 而不是进 server 模式
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            tokimo_bus_cli::print_help_unified(&mut cmd);
            std::process::exit(0);
        }
        Some(cmd) => {
            // CLI 模式：纯文本错误，不输出 tracing 日志
            let result = match cmd {
                Command::Items { cmd: None } => {
                    use clap::CommandFactory;
                    let mut root = Cli::command();
                    root.build();
                    if let Some(items_cmd) = root.find_subcommand_mut("items") {
                        tokimo_bus_cli::print_help_unified(items_cmd);
                    }
                    std::process::exit(0);
                }
                Command::Items { cmd: Some(c) } => cli::run_items(auth, c).await,
                Command::Greet { name } => cli::run_greet(auth, name).await,
            };
            if let Err(error) = result {
                eprintln!("Error: {error:#}");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

async fn run_server() -> anyhow::Result<()> {
    let cfg = ClientConfig::from_env().map_err(|e| anyhow::anyhow!("ClientConfig: {e}"))?;
    info!(endpoint = ?cfg.endpoint, "helloworld: connecting to broker");

    let db = db::init_pool().await?;
    info!("helloworld: db connected (schema managed by host)");

    // BusClient 仍然存在 —— 不为暴露方法，而是：
    // 1) 让 broker 知道 helloworld 在线（supervisor 健康检查）
    // 2) 提供 cross-app `bus.call("notification_center", "notify", ...)` 通道
    let client_slot: Arc<OnceLock<Arc<BusClient>>> = Arc::new(OnceLock::new());
    let ctx = Arc::new(handlers::AppCtx {
        db,
        client: Arc::clone(&client_slot),
    });

    // 起 axum router 监听 UDS（业务 + assets + data 都在这个 sock 上）
    let app_socket = app_server::spawn("helloworld", Arc::clone(&ctx))
        .await
        .map_err(|e| anyhow::anyhow!("app_server spawn: {e}"))?;

    // 把 sock 通过 `data_plane_socket` 上报给 broker（server 用它做反代目的地）
    let client = BusClient::builder(cfg)
        .service("helloworld", env!("CARGO_PKG_VERSION"))
        .data_plane(app_socket)
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("bus build: {e}"))?;
    client_slot
        .set(Arc::clone(&client))
        .map_err(|_| anyhow::anyhow!("client_slot already set"))?;

    info!("helloworld: registered with broker");

    let shutdown = {
        let client = Arc::clone(&client);
        tokio::spawn(async move { client.run_until_shutdown().await })
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("helloworld: SIGINT received");
            client.shutdown();
        }
        _ = shutdown => info!("helloworld: broker sent Shutdown"),
    }

    Ok(())
}
