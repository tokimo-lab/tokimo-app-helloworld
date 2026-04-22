//! UI asset serving.
//!
//! Production: assets are bundled via `rust-embed!` at build time.
//! Dev override: if `TOKIMO_APP_ASSETS_DIR` is set, files are read from disk
//!  so the app author can iterate on the UI without rebuilding cargo.

use rust_embed::RustEmbed;
use serde::Deserialize;
use tokimo_bus_protocol::BusError;

#[derive(RustEmbed)]
#[folder = "ui/dist/"]
#[prefix = ""]
struct EmbeddedUi;

#[derive(Deserialize)]
struct GetReq {
    path: String,
}

pub async fn handle(payload: Vec<u8>) -> Result<Vec<u8>, BusError> {
    let GetReq { path } = serde_json::from_slice(&payload)
        .map_err(|e| BusError::BadRequest(format!("assets.get: {e}")))?;

    let normalised = normalise(&path);

    if let Ok(dir) = std::env::var("TOKIMO_APP_ASSETS_DIR") {
        let full = std::path::Path::new(&dir).join(&normalised);
        return tokio::fs::read(&full)
            .await
            .map_err(|e| BusError::Internal(format!("asset {normalised}: {e}")));
    }

    EmbeddedUi::get(&normalised)
        .map(|f| f.data.into_owned())
        .ok_or_else(|| BusError::Internal(format!("embedded asset not found: {normalised}")))
}

fn normalise(path: &str) -> String {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() || trimmed.ends_with('/') {
        format!("{trimmed}index.html")
    } else {
        trimmed.to_string()
    }
}
