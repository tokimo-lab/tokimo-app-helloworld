/**
 * Hello World sample app — exhaustive demo / playground for @tokimo/sdk.
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
  makeTranslator,
  RuntimeProvider,
} from "@tokimo/sdk";
import {
  ConfigProvider,
  Input,
  ToastProvider,
  enUS as uiEnUS,
  zhCN as uiZhCN,
} from "@tokimo/ui";
import { Search, Sparkles } from "lucide-react";
import { StrictMode, useMemo, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import {
  AppearanceDemo,
  CtxDemo,
  MenuBarDemo,
  NotifyDemo,
  ToastDemo,
  WindowNavDemo,
} from "./demos/BasicDemos";
import { ItemsCrudDemo } from "./demos/ItemsCrudDemo";
import { BulkImportJobDemo, LongRunningJobDemo } from "./demos/JobsDemos";
import { MediaCenterSnapshotDemo, MediaSessionDemo } from "./demos/MediaDemos";
import type { DemoEntry } from "./demos/shared";
import { enUS, zhCN } from "./i18n";
import "./index.css";
import { ViewerDemoPanel } from "./viewer-demo";

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
    id: "media-snapshot",
    category: "Media",
    title: "useMediaCenter (snapshot)",
    api: "useMediaCenter(ctx).snapshot",
    Component: MediaCenterSnapshotDemo,
  },
  {
    id: "media-session",
    category: "Media",
    title: "useMediaCenter (controls)",
    api: "useMediaCenter(ctx).api",
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
  {
    id: "bulk-import-job",
    category: "Jobs",
    title: "Bulk import job",
    api: "POST /api/apps/helloworld/jobs/start",
    Component: BulkImportJobDemo,
  },
  {
    id: "long-running-job",
    category: "Jobs",
    title: "Long running job",
    api: "POST /api/apps/helloworld/jobs/start",
    Component: LongRunningJobDemo,
  },
  {
    id: "viewer-demo",
    category: "Demo App",
    title: "Viewer panel",
    api: "import { ... } from '@tokimo/sdk/viewers'",
    Component: ViewerDemoPanel,
  },
];

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
    <div className="flex h-full w-full text-[var(--color-fg-primary)]">
      <aside className="flex w-[240px] flex-col border-r border-black/10 dark:border-white/10 bg-black/[0.02] dark:bg-white/[0.03]">
        <div className="flex items-center gap-2 border-b border-black/10 dark:border-white/10 px-3 py-3">
          <Sparkles size={18} style={{ color: "var(--color-accent)" }} />
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
                        ? "bg-[var(--color-accent-subtle)] text-[var(--color-accent)]"
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

      <main className="flex-1 overflow-auto">
        <header className="sticky top-0 z-10 border-b border-black/10 dark:border-white/10 bg-surface-base/80 dark:bg-black/40 backdrop-blur px-6 py-4">
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
    root.render(
      <StrictMode>
        <ConfigProvider locale={locale}>
          <ToastProvider>
            <RuntimeProvider value={ctx}>
              <HelloworldWindow ctx={ctx} />
            </RuntimeProvider>
          </ToastProvider>
        </ConfigProvider>
      </StrictMode>,
    );
    return () => root.unmount();
  },
});
