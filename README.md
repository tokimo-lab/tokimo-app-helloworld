# tokimo-app-helloworld

Reference Tokimo app — minimal "hello world" template for the multi-process
app architecture (axum-on-UDS + transparent reverse proxy via [`tokimo-bus`][bus]).
Use this as a starting point when writing your own third-party app.

## Architecture

```
Browser
  │  /api/apps/helloworld/<route>
  ▼
tokimo-server (5678)        — auth、CORS、注入 X-Tokimo-User-Id 等 header
  │  透明反代 → UDS
  ▼
$DATA_LOCAL_PATH/apps/helloworld.sock
  │
this binary
  ├─ axum router (src/app_server.rs)         全部路由挂在同一个 sock 上
  │   ├─ GET/POST  /items                    CRUD
  │   ├─ DELETE    /items/{id}
  │   ├─ POST      /items/notify             跨 app 调 notification_center
  │   ├─ POST      /greet                    typed JSON 演示
  │   ├─ POST      /echo                     透传 body
  │   ├─ GET       /assets/{*path}           静态资源（rust-embed）
  │   └─ GET       /data/hello.txt           数据流示例
  ├─ tokimo-bus client                       仅向 broker 上报 sock + 跨 app 调用
  └─ Postgres direct (schema=helloworld)     启动跑 migrations/0001_init.sql
```

## What it shows

- 标准 axum handler 签名（`State<Arc<AppCtx>>` / `Json<Req>` / `Result<_, AppError>`）
- 从 `x-tokimo-user-id` header 取用户身份（server 反代时注入）
- `BusClient::builder().service(...).data_plane(socket)` —— 仅注册自己 + 上报 sock，不再
  逐个 `.method().on_invoke()`
- 跨 app 调用：`items_add_with_notify` 通过 `BusClient.invoke("notification_center", "notify", ...)`
- rust-embed 嵌入 `ui/dist`，dev 模式下 `TOKIMO_APP_ASSETS_DIR_*` 走文件系统
- 优雅关闭：SIGINT 或 broker `Shutdown` 帧

## 本地开发循环

### 改 Rust

```bash
cargo build -p tokimo-app-helloworld
# supervisor 不会自动检测 binary mtime，需手动 kill 让它 respawn：
pkill -f tokimo-app-helloworld
```

### 改 UI（不用 cargo build）

`scripts/dev.sh` 已通过 `tokimo-app.toml` 的 `runtime.ui_dist` 字段为每个 app 注入
`TOKIMO_APP_ASSETS_DIR`，资源 handler 优先读文件系统而不是 embed。

```bash
pnpm -C apps/tokimo-app-helloworld/ui build --watch
# 浏览器强刷即可生效
```

## License

MIT OR Apache-2.0.

[bus]: https://github.com/tokimo-lab/tokimo-bus
