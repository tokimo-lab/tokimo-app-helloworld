//! 数据面：本地 Unix socket HTTP 服务器，给 server 反代调用。
//!
//! 约定：socket 路径 = `dirname($TOKIMO_BUS_SOCKET)/apps/<service>.data.sock`。
//! Server 侧 `/api/apps/helloworld/data/<path>` 会反代到这里，
//! `<path>` 成为请求 URI 的 path（不含前缀）。

use std::{convert::Infallible, path::PathBuf};

use http_body_util::Full;
use hyper::{Request, Response, body::Bytes, service::service_fn};
use hyper_util::rt::TokioIo;
use tokimo_bus_protocol::DataPlaneSocket;
use tokio::net::UnixListener;
use tracing::{debug, error, info};

/// 根据 broker socket 路径推出同 app 的数据面 socket 路径。
pub fn default_socket_path(service: &str) -> anyhow::Result<PathBuf> {
    let bus = std::env::var("TOKIMO_BUS_SOCKET")
        .map_err(|_| anyhow::anyhow!("TOKIMO_BUS_SOCKET not set"))?;
    let parent = PathBuf::from(&bus)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("TOKIMO_BUS_SOCKET has no parent"))?
        .to_path_buf();
    let apps_dir = parent.join("apps");
    std::fs::create_dir_all(&apps_dir)?;
    Ok(apps_dir.join(format!("{service}.data.sock")))
}

/// 在后台起 hyper HTTP/1.1 server；返回该 socket 的 `DataPlaneSocket`，
/// 交给 `BusClientBuilder::data_plane(...)` 上报给 broker。
pub async fn spawn(service: &str) -> anyhow::Result<DataPlaneSocket> {
    let path = default_socket_path(service)?;
    // socket 文件残留 → 删掉再 bind
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    info!(path = %path.display(), "helloworld data-plane listening");

    tokio::spawn(async move {
        loop {
            let (stream, _addr) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    error!(error = %e, "data-plane accept failed");
                    continue;
                }
            };
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, service_fn(handle))
                    .await
                {
                    debug!(error = %err, "data-plane conn ended");
                }
            });
        }
    });

    Ok(DataPlaneSocket::Unix {
        path: path.to_string_lossy().into_owned(),
    })
}

async fn handle(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    match path {
        "/hello.txt" => Ok(Response::builder()
            .status(200)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Full::new(Bytes::from_static(
                b"hello from helloworld data-plane\n",
            )))
            .unwrap()),
        _ => Ok(Response::builder()
            .status(404)
            .body(Full::new(Bytes::from_static(b"not found")))
            .unwrap()),
    }
}
