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
  │   ├─ GET/POST  /items                    列表 / 新增
  │   ├─ PUT/DELETE /items/{id}              更新 / 删除（PUT 需认证）
  │   ├─ POST      /items/notify             认证后新增并跨 app 调 notification_center.notify
  │   ├─ POST      /greet                    typed JSON 演示
  │   ├─ POST      /echo                     透传 body
  │   ├─ GET       /assets/{*path}           静态资源（rust-embed）
  │   └─ GET       /data/hello.txt           数据流示例
  ├─ tokimo-bus client                       仅向 broker 上报 sock + 跨 app 调用
  └─ Postgres direct (schema=helloworld)     启动跑 migrations/0001_init.sql
```

## What it shows

- 标准 axum handler 签名（`State<Arc<AppCtx>>` / `Json<Req>` / `Result<_, AppError>`）
- `TokimoUser` extractor 从 server 反代注入的 `x-tokimo-user-id` 读取用户身份
- `PUT /items/{id}` 演示认证后的更新接口
- `BusClient::builder().service(...).data_plane(socket)` —— 仅注册自己 + 上报 sock，不再
  逐个 `.method().on_invoke()`
- 跨 app 调用：`items_add_with_notify` 认证后通过 `BusClient.invoke("notification_center", "notify", ...)` 发通知
- rust-embed 嵌入 `ui/dist`，dev 模式下 `TOKIMO_APP_ASSETS_DIR_*` 走文件系统
- 优雅关闭：SIGINT 或 broker `Shutdown` 帧

## CLI 用法

前置条件：

1. 启动 Tokimo 主 server（默认 `http://localhost:5678`）。
2. 浏览器登录后，在「设置 → API Keys」创建一个 `mm_xxx` token。
3. 通过 `--tokimo-token` 或 `TOKIMO_TOKEN` 环境变量传入 token。

### `tokimo-app-helloworld --help`

```text
Helloworld 是 Tokimo 子 app 的样板，演示：
- server 模式（被主 server supervisor 拉起，无参运行）
- CLI 模式（带子命令运行，使用 --tokimo-token mm_xxx 鉴权）

CLI 用法前置条件：
1. 启动 Tokimo 主 server (默认 http://localhost:5678)
2. 浏览器登录后，去「设置 → API Keys」创建一个 token (mm_xxx)
3. 把 token 通过 --tokimo-token 或 TOKIMO_TOKEN env 传入

Usage: tokimo-app-helloworld.exe [OPTIONS] [COMMAND]

Commands:
  serve  启动 server 模式（无参运行时默认行为）
  items  管理 helloworld items
  greet  调用 helloworld 的 POST /greet。
  help   Print this message or the help of the given subcommand(s)

Options:
      --tokimo-token <TOKEN>
          Tokimo API token (mm_xxx). 在主 server 的设置页 → API Keys 创建。

          [env: TOKIMO_TOKEN]
      --tokimo-server <SERVER>
          Tokimo 主 server URL，默认 http://localhost:5678

          [env: TOKIMO_SERVER_URL=]
  -h, --help
          Print help (see a summary with '-h')
```

### `tokimo-app-helloworld items --help`

```text
管理 helloworld items

Usage: tokimo-app-helloworld.exe items [OPTIONS] <COMMAND>

Commands:
  list    列出最近 100 条 item。
  add     新增一条 item。
  update  更新指定 item 的 content。
  delete  删除指定 item。
  help    Print this message or the help of the given subcommand(s)

Options:
      --tokimo-token <TOKEN>
          Tokimo API token (mm_xxx). 在主 server 的设置页 → API Keys 创建。

          [env: TOKIMO_TOKEN]
      --tokimo-server <SERVER>
          Tokimo 主 server URL，默认 http://localhost:5678

          [env: TOKIMO_SERVER_URL=]
  -h, --help
          Print help (see a summary with '-h')
```

### 示例

```powershell
.\tokimo-app-helloworld.exe serve
.\tokimo-app-helloworld.exe --tokimo-token mm_xxx greet Alice
.\tokimo-app-helloworld.exe --tokimo-token mm_xxx items list
.\tokimo-app-helloworld.exe --tokimo-token mm_xxx items add "hello tokimo"
.\tokimo-app-helloworld.exe --tokimo-token mm_xxx items update 018f0000-0000-7000-8000-000000000000 "updated content"
.\tokimo-app-helloworld.exe --tokimo-token mm_xxx items delete 018f0000-0000-7000-8000-000000000000
$env:TOKIMO_TOKEN = "mm_xxx"; .\tokimo-app-helloworld.exe items list
```

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

#### UI 构建配置：`@tokimo/app-builder`

`ui/vite.config.ts` 只有一行 `defineTokimoApp()`，完整的 library 模式 + externals
配置由共享预设 [`@tokimo/app-builder`](https://github.com/tokimo-lab/tokimo)
（主仓 `packages/tokimo-app-builder/`）提供：

- **externals**：`react` / `react-dom` / `@tokimo/ui` / `@tokimo/sdk` 全部不打进 bundle，
  由主 shell 通过 `<script type="importmap">` + `window.__TKM_DEPS__` 注入同一份实例
  （否则跨边界 React hooks 会断）
- **产物**：`dist/index.js` + `dist/index.css`，被主 server 反代到 `/api/apps/<id>/assets/`

如需特殊 vite 配置（额外 plugin / overrides），传入 options：

```ts
import { defineTokimoApp } from "@tokimo/app-builder/vite";
export default defineTokimoApp({ extraExternal: ["some-shared-lib"] });
```

### 独立开发（不依赖主仓）

这个 app 可以脱离 tokimo 主仓 clone 后直接 dev：

```bash
git clone git@github.com:tokimo-lab/tokimo-app-helloworld.git
cd tokimo-app-helloworld/ui
pnpm install   # 拉 @tokimo/ui / @tokimo/sdk / @tokimo/app-builder 的 git 源码
pnpm dev       # vite watch
```

主仓内开发时，`ui/.pnpmfile.cjs` 会自动检测主仓上下文（往上找
`packages/tokimo-app-builder/package.json`），把这三个依赖改写为 `file:`
路径直接 link 到主仓 submodule，无需 bump sha 即可看到 ui/sdk 的修改。

## License

MIT OR Apache-2.0.

[bus]: https://github.com/tokimo-lab/tokimo-bus
