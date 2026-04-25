/**
 * Hello World sample app — exhaustive demo / playground for @tokimo/app-sdk.
 *
 * Layout: Master-Detail. Left sidebar is a searchable, categorized list of
 * "demos"; right area renders the selected demo's description + live state
 * snapshot + interactive controls.
 *
 * To add a new API demo: append one entry to the DEMOS array. No layout edits.
 */
import {
  type AppRuntimeCtx,
  type Dispose,
  defineApp,
  type MenuBarConfig,
  makeTranslator,
} from "@tokimo/app-sdk";
import {
  useShellAppearance,
  useShellMedia,
  useShellMediaSessionSnapshot,
  useShellMenuBar,
  useShellToast,
  useShellWindowNav,
} from "@tokimo/app-sdk/react";
import {
  Button,
  Card,
  ConfigProvider,
  Empty,
  Input,
  ToastProvider,
  enUS as uiEnUS,
  zhCN as uiZhCN,
} from "@tokimo/ui";
import { Search, Sparkles, Trash2 } from "lucide-react";
import {
  type ReactNode,
  StrictMode,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { createRoot, type Root } from "react-dom/client";
import { enUS, zhCN } from "./i18n";
import "./index.css";

interface Item {
  id: string;
  content: string;
  created_at: string;
}

const SERVICE = "helloworld";

function fmt(v: unknown): string {
  try {
    return JSON.stringify(
      v,
      (_k, val) => {
        if (typeof val === "function") return "[fn]";
        if (val instanceof Element) return "[Element]";
        return val;
      },
      2,
    );
  } catch {
    return String(v);
  }
}

// ── Demo registry ──────────────────────────────────────────────────────────
//
// Each demo is a self-contained component that consumes ctx + t. The sidebar
// is generated from this list — to add a new API showcase, append one entry.

interface DemoEntry {
  id: string;
  category: string;
  title: string;
  api: string; // hook / call signature shown as code
  Component: React.FC<{ ctx: AppRuntimeCtx; t: (k: string) => string }>;
}

function CtxDemo({ ctx }: { ctx: AppRuntimeCtx; t: (k: string) => string }) {
  return (
    <Section
      desc="Mount-time context. Captured once at mount; for reactive values prefer the corresponding useShell* hook."
      code="function mount(el, ctx: AppRuntimeCtx) { ... }"
    >
      <Snapshot>
        {fmt({
          windowId: ctx.windowId,
          appId: ctx.appId,
          locale: ctx.locale,
          theme: ctx.theme,
        })}
      </Snapshot>
    </Section>
  );
}

function AppearanceDemo({ ctx }: { ctx: AppRuntimeCtx }) {
  const ap = useShellAppearance(ctx);
  useEffect(() => {
    console.log("[helloworld] appearance →", ap);
  }, [ap]);
  return (
    <Section
      desc="Reactive theme + title-bar style. Use this instead of ctx.theme so apps re-render when the user toggles dark mode or window chrome."
      code="const ap = useShellAppearance(ctx);"
    >
      <Snapshot>{fmt(ap)}</Snapshot>
    </Section>
  );
}

function WindowNavDemo({ ctx }: { ctx: AppRuntimeCtx }) {
  const nav = useShellWindowNav(ctx);
  useEffect(() => {
    console.log("[helloworld] windowNav →", {
      route: nav.route,
      canGoBack: nav.canGoBack,
    });
  }, [nav.route, nav.canGoBack]);
  return (
    <Section
      desc="Per-window route stack persisted in DB — F5 restores the route. navigate pushes; replace overwrites; goBack pops."
      code="const nav = useShellWindowNav(ctx);"
    >
      <Snapshot>{fmt({ route: nav.route, canGoBack: nav.canGoBack })}</Snapshot>
      <ButtonRow>
        <Button
          size="small"
          onClick={() => nav.navigate(`/items/${Date.now()}`, "Item")}
        >
          navigate
        </Button>
        <Button size="small" onClick={() => nav.replace("/")}>
          replace /
        </Button>
        <Button
          size="small"
          variant="default"
          disabled={!nav.canGoBack}
          onClick={() => nav.goBack()}
        >
          back
        </Button>
      </ButtonRow>
    </Section>
  );
}

function ToastDemo({ ctx }: { ctx: AppRuntimeCtx }) {
  const toast = useShellToast(ctx);
  return (
    <Section
      desc="In-window toast notifications (stateless). Four levels: info, success, warning, error."
      code="const toast = useShellToast(ctx); toast.success('hi');"
    >
      <ButtonRow>
        <Button size="small" onClick={() => toast.info("info toast")}>
          info
        </Button>
        <Button size="small" onClick={() => toast.success("success toast")}>
          success
        </Button>
        <Button size="small" onClick={() => toast.warning("warning toast")}>
          warning
        </Button>
        <Button size="small" onClick={() => toast.error("error toast")}>
          error
        </Button>
      </ButtonRow>
    </Section>
  );
}

function MediaDemo({ ctx }: { ctx: AppRuntimeCtx }) {
  const media = useShellMedia(ctx);
  useEffect(() => {
    console.log("[helloworld] media.snapshot →", media.snapshot);
  }, [media.snapshot]);
  return (
    <Section
      desc="Central audio engine snapshot. Reactive — re-renders when any other app plays / pauses."
      code="const media = useShellMedia(ctx); media.snapshot.isPlaying;"
    >
      <Snapshot>
        {fmt({
          isPlaying: media.snapshot.isPlaying,
          currentTime: media.snapshot.currentTime,
          duration: media.snapshot.duration,
          volume: media.snapshot.volume,
          activeProvider: media.snapshot.activeProvider,
        })}
      </Snapshot>
    </Section>
  );
}

function MediaSessionDemo({ ctx }: { ctx: AppRuntimeCtx }) {
  const session = useShellMediaSessionSnapshot(ctx);
  useEffect(() => {
    console.log("[helloworld] mediaSession →", session);
  }, [session]);
  return (
    <Section
      desc="Cross-app 'now playing' source. Reads who's currently the active media session across the desktop. Useful for global media controls."
      code="const s = useShellMediaSessionSnapshot(ctx); s.activeSource"
    >
      <Snapshot>
        {fmt({
          activeSource: session.activeSource
            ? {
                id: session.activeSource.id,
                type: session.activeSource.type,
                provider: session.activeSource.provider,
                title: session.activeSource.title,
                artist: session.activeSource.artist,
                isPlaying: session.activeSource.isPlaying,
              }
            : null,
          rawPlaybackDataReady: session.rawPlaybackDataReady,
        })}
      </Snapshot>
    </Section>
  );
}

function NotifyDemo({
  ctx,
  t,
}: {
  ctx: AppRuntimeCtx;
  t: (k: string) => string;
}) {
  const fire = useCallback(async () => {
    await ctx.shell.notify({
      categoryId: "manual",
      categoryLabel: "helloworld.notifications.manual",
      title: t("notifyTitle"),
      body: t("notifyBody"),
      level: "info",
    });
  }, [ctx.shell, t]);
  return (
    <Section
      desc="Stateless, fire-and-forget. Posts to the notification center which broadcasts via WebSocket. No useShell* wrapper because there's no reactive state."
      code="await ctx.shell.notify({ title, body, level })"
    >
      <ButtonRow>
        <Button size="small" variant="primary" onClick={fire}>
          send notification
        </Button>
      </ButtonRow>
    </Section>
  );
}

function MenuBarDemo({
  ctx,
  t,
}: {
  ctx: AppRuntimeCtx;
  t: (k: string) => string;
}) {
  const toast = useShellToast(ctx);
  const nav = useShellWindowNav(ctx);
  const config = useMemo<MenuBarConfig>(
    () => ({
      menus: [
        {
          key: "helloworld",
          label: t("menuLabel"),
          items: [
            {
              key: "toast",
              label: t("menuToast"),
              onClick: () => toast.info(t("menuToastMsg")),
            },
            {
              key: "notify",
              label: t("menuNotify"),
              onClick: async () => {
                await ctx.shell.notify({
                  categoryId: "menu",
                  title: t("notifyTitle"),
                  body: t("notifyFromMenu"),
                  level: "info",
                });
              },
            },
            { type: "divider" as const },
            {
              key: "back",
              label: t("menuGoBack"),
              disabled: !nav.canGoBack,
              onClick: () => nav.goBack(),
            },
          ],
        },
      ],
      about: { description: t("subtitle"), version: "0.1.0" },
    }),
    [ctx.shell, nav, toast, t],
  );
  useShellMenuBar(ctx, config);
  return (
    <Section
      desc="Registers the top menubar while this window is focused. Auto-unregisters on unmount."
      code="useShellMenuBar(ctx, config)"
    >
      <p className="text-xs opacity-70">{t("menuHint")}</p>
    </Section>
  );
}

function ItemsCrudDemo({
  ctx: _ctx,
  t,
}: {
  ctx: AppRuntimeCtx;
  t: (k: string) => string;
}) {
  const [items, setItems] = useState<Item[]>([]);
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const r = await fetch(`/api/apps/${SERVICE}/items`, {
        credentials: "include",
      });
      if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
      const res = (await r.json()) as { items: Item[] };
      setItems(res.items ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const add = useCallback(
    async (notify: boolean) => {
      const text = content.trim();
      if (!text) return;
      setError(null);
      try {
        const url = notify
          ? `/api/apps/${SERVICE}/items/notify`
          : `/api/apps/${SERVICE}/items`;
        const r = await fetch(url, {
          method: "POST",
          credentials: "include",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ content: text }),
        });
        if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
        setContent("");
        await refresh();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [content, refresh],
  );

  const remove = useCallback(
    async (id: string) => {
      try {
        const r = await fetch(
          `/api/apps/${SERVICE}/items/${encodeURIComponent(id)}`,
          { method: "DELETE", credentials: "include" },
        );
        if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
        await refresh();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [refresh],
  );

  return (
    <Section
      desc="The original Rust-backed CRUD demo. Validates the multi-process app architecture (Rust handler in src/, accessed via /api/apps/helloworld/*)."
      code="GET/POST/DELETE /api/apps/helloworld/items"
    >
      <Card className="p-3">
        <div className="flex flex-wrap gap-2">
          <Input
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder={t("inputPlaceholder")}
            onKeyDown={(e) => {
              if (e.key === "Enter") void add(false);
            }}
          />
          <Button onClick={() => add(false)}>{t("add")}</Button>
          <Button variant="primary" onClick={() => add(true)}>
            {t("addAndNotify")}
          </Button>
          <Button variant="default" onClick={refresh}>
            {t("refresh")}
          </Button>
        </div>
        {error && (
          <div className="mt-2 text-sm text-red-500">
            {t("error")}
            {error}
          </div>
        )}
      </Card>
      <Card className="p-3">
        {loading ? (
          <div className="opacity-60">{t("loading")}</div>
        ) : items.length === 0 ? (
          <Empty description={t("empty")} />
        ) : (
          <ul className="flex flex-col gap-2">
            {items.map((it) => (
              <li
                key={it.id}
                className="flex items-center justify-between rounded border border-black/10 px-3 py-2 dark:border-white/10"
              >
                <div className="flex flex-col">
                  <span>{it.content}</span>
                  <span className="text-xs opacity-50">{it.created_at}</span>
                </div>
                <Button
                  variant="default"
                  size="small"
                  onClick={() => remove(it.id)}
                >
                  <Trash2 size={14} /> {t("delete")}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </Section>
  );
}

const DEMOS: DemoEntry[] = [
  {
    id: "ctx",
    category: "App Lifecycle",
    title: "ctx (mount-time)",
    api: "AppRuntimeCtx",
    Component: CtxDemo,
  },
  {
    id: "appearance",
    category: "Appearance",
    title: "useShellAppearance",
    api: "useShellAppearance(ctx)",
    Component: AppearanceDemo,
  },
  {
    id: "window-nav",
    category: "Window",
    title: "useShellWindowNav",
    api: "useShellWindowNav(ctx)",
    Component: WindowNavDemo,
  },
  {
    id: "toast",
    category: "UI",
    title: "useShellToast",
    api: "useShellToast(ctx)",
    Component: ToastDemo,
  },
  {
    id: "menubar",
    category: "UI",
    title: "useShellMenuBar",
    api: "useShellMenuBar(ctx, config)",
    Component: MenuBarDemo,
  },
  {
    id: "media",
    category: "Media",
    title: "useShellMedia",
    api: "useShellMedia(ctx)",
    Component: MediaDemo,
  },
  {
    id: "media-session",
    category: "Media",
    title: "useShellMediaSessionSnapshot",
    api: "useShellMediaSessionSnapshot(ctx)",
    Component: MediaSessionDemo,
  },
  {
    id: "notify",
    category: "Notifications",
    title: "ctx.shell.notify",
    api: "ctx.shell.notify(payload)",
    Component: NotifyDemo,
  },
  {
    id: "items-crud",
    category: "Demo App",
    title: "Items CRUD",
    api: "/api/apps/helloworld/items",
    Component: ItemsCrudDemo,
  },
];

// ── Layout primitives ──────────────────────────────────────────────────────

function Section({
  desc,
  code,
  children,
}: {
  desc: string;
  code: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm opacity-80 leading-relaxed">{desc}</p>
      <pre className="rounded bg-black/[0.06] dark:bg-white/[0.06] px-3 py-2 text-xs font-mono">
        {code}
      </pre>
      {children}
    </div>
  );
}

function Snapshot({ children }: { children: ReactNode }) {
  return (
    <pre className="rounded border border-black/10 bg-white/60 dark:border-white/10 dark:bg-black/40 px-3 py-2 text-xs leading-snug whitespace-pre-wrap break-words">
      {children}
    </pre>
  );
}

function ButtonRow({ children }: { children: ReactNode }) {
  return <div className="flex flex-wrap gap-2">{children}</div>;
}

// ── Main window ────────────────────────────────────────────────────────────

function HelloworldWindow({ ctx }: { ctx: AppRuntimeCtx }) {
  const t = makeTranslator({ "zh-CN": zhCN, "en-US": enUS }, ctx.locale);
  const [selectedId, setSelectedId] = useState<string>("appearance");
  const [query, setQuery] = useState("");

  const grouped = useMemo(() => {
    const q = query.trim().toLowerCase();
    const filtered = q
      ? DEMOS.filter(
          (d) =>
            d.title.toLowerCase().includes(q) ||
            d.api.toLowerCase().includes(q) ||
            d.category.toLowerCase().includes(q),
        )
      : DEMOS;
    const map = new Map<string, DemoEntry[]>();
    for (const d of filtered) {
      const list = map.get(d.category) ?? [];
      list.push(d);
      map.set(d.category, list);
    }
    return Array.from(map.entries());
  }, [query]);

  const current = DEMOS.find((d) => d.id === selectedId) ?? DEMOS[0];
  const Demo = current.Component;

  return (
    <div className="flex h-full w-full text-[var(--text-primary)]">
      {/* Sidebar */}
      <aside className="flex w-[240px] flex-col border-r border-black/10 dark:border-white/10 bg-black/[0.02] dark:bg-white/[0.03]">
        <div className="flex items-center gap-2 border-b border-black/10 dark:border-white/10 px-3 py-3">
          <Sparkles size={18} style={{ color: "var(--accent)" }} />
          <div className="flex flex-col">
            <span className="text-sm font-semibold">{t("title")}</span>
            <span className="text-[10px] opacity-60">{t("subtitle")}</span>
          </div>
        </div>
        <div className="px-2 py-2">
          <div className="relative">
            <Search
              size={12}
              className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 opacity-50 z-10"
            />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("searchPlaceholder")}
              className="w-full pl-7"
              size="small"
            />
          </div>
        </div>
        <nav className="flex-1 overflow-auto px-1 py-1">
          {grouped.length === 0 && (
            <div className="px-3 py-2 text-xs opacity-50">{t("noMatch")}</div>
          )}
          {grouped.map(([cat, demos]) => (
            <div key={cat} className="mb-2">
              <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wide opacity-50">
                {cat}
              </div>
              {demos.map((d) => {
                const active = d.id === selectedId;
                return (
                  <button
                    key={d.id}
                    type="button"
                    onClick={() => setSelectedId(d.id)}
                    className={`w-full cursor-pointer rounded px-2 py-1.5 text-left text-xs transition ${
                      active
                        ? "bg-[var(--accent-subtle)] text-[var(--accent)]"
                        : "hover:bg-black/[0.05] dark:hover:bg-white/[0.05]"
                    }`}
                  >
                    {d.title}
                  </button>
                );
              })}
            </div>
          ))}
        </nav>
      </aside>

      {/* Detail */}
      <main className="flex-1 overflow-auto">
        <header className="sticky top-0 z-10 border-b border-black/10 dark:border-white/10 bg-[var(--surface,white)]/80 dark:bg-black/40 backdrop-blur px-6 py-4">
          <div className="text-[10px] uppercase tracking-wide opacity-50">
            {current.category}
          </div>
          <h1 className="text-lg font-semibold">{current.title}</h1>
          <code className="text-xs opacity-70">{current.api}</code>
        </header>
        <div className="px-6 py-5">
          <Demo ctx={ctx} t={t} />
        </div>
      </main>
    </div>
  );
}

export default defineApp({
  id: "helloworld",
  manifest: {
    id: "helloworld",
    appName: "Hello World",
    icon: "Sparkles",
    image: "icon.png",
    color: "#10b981",
    windowType: "helloworld",
    defaultSize: { width: 1080, height: 660 },
    category: "app",
  },
  translations: { "zh-CN": zhCN, "en-US": enUS },
  mount(container, ctx): Dispose {
    const root: Root = createRoot(container);
    const locale = ctx.locale.startsWith("zh") ? uiZhCN : uiEnUS;
    // Theme mode (dark/light) and accent color cascade from the host's
    // `<html>` via CSS variables — no `theme` prop here, otherwise our
    // bundle would fight the host for the global `data-accent` / `.dark`
    // attribute. Use `var(--accent)` etc. in styles to follow live changes.
    root.render(
      <StrictMode>
        <ConfigProvider locale={locale}>
          <ToastProvider>
            <HelloworldWindow ctx={ctx} />
          </ToastProvider>
        </ConfigProvider>
      </StrictMode>,
    );
    return () => root.unmount();
  },
});
