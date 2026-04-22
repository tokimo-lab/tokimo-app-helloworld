# tokimo-app-helloworld

Reference Tokimo app: the "hello world" of [`tokimo-bus`][bus]. Use this
as a template when writing your own third-party app.

## What it shows

- Connecting to the broker with `ClientConfig::from_env()` (reads
  `TOKIMO_BUS_SOCKET` + `TOKIMO_BUS_TOKEN` that the `tokimo-bus-supervisor`
  injects before spawning you).
- Declaring methods the main server can invoke:
  - `echo` — returns the request payload unchanged.
  - `greet` — accepts `{ "name": "..." }` and returns `{ "message": "Hello, ..." }`.
  - `items.list` / `items.add` / `items.remove` / `items.add_with_notify` — CRUD + cross-app notify.
- Starting a **data-plane UDS server** (`src/data_plane.rs`) reachable from the
  browser at `GET /api/apps/helloworld/data/hello.txt` — stream-to-stream reverse
  proxied by the server, bypassing the bus frame protocol. This is the channel
  video / large files use; bus frames carry only JSON control messages.
- Publishing a periodic `helloworld.heartbeat` event that any subscriber
  (e.g. monitoring app) can listen to.
- Graceful shutdown on SIGINT or broker-initiated Shutdown frame.

## Run it locally

```bash
cargo build --release
```

The binary expects the supervisor to hand it environment variables, so
you normally don't launch it by hand. For quick experimentation, start a
tiny standalone broker:

```rust,ignore
use tokimo_bus_broker::{Broker, BrokerConfig};
let broker = Broker::new(BrokerConfig::default());
broker.listen_unix("/tmp/tokimo-bus.sock").await?;
let token = broker.issue_token("helloworld");
// export TOKIMO_BUS_SOCKET=/tmp/tokimo-bus.sock TOKIMO_BUS_TOKEN=...
```

Then in another terminal:

```bash
TOKIMO_BUS_SOCKET=/tmp/tokimo-bus.sock \
TOKIMO_BUS_TOKEN=<token from broker> \
./target/release/tokimo-app-helloworld
```

## License

MIT OR Apache-2.0.

[bus]: https://github.com/tokimo-lab/tokimo-bus
