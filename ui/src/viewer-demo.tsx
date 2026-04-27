import {
  AudioPlayer,
  BookViewer,
  type EpubBook,
  EpubViewer,
  HexViewer,
  HtmlPreview,
  ImagePreview,
  MonacoTextEditor,
  PdfEmbed,
  VideoPreview,
} from "@tokimo/sdk/viewers";
import { useCallback, useMemo, useState } from "react";
import {
  createDemoPdfBlob,
  createDemoWavBlob,
  fetchDemoChapter,
  fetchDemoEpub,
  fetchDemoHexRange,
  HTML_SAMPLE,
  IMAGE_DATA_URL,
  parseDemoEpub,
  TEXT_SAMPLE,
  useBlobUrl,
  useGeneratedVideoUrl,
} from "./viewer-demo-samples";

type ViewerDemoId =
  | "text"
  | "image"
  | "audio"
  | "video"
  | "pdf"
  | "html"
  | "hex"
  | "book"
  | "epub";

interface ViewerDemoTab {
  id: ViewerDemoId;
  label: string;
  summary: string;
}

const VIEWER_DEMO_TABS: ViewerDemoTab[] = [
  { id: "text", label: "Text / Code", summary: "read-only TypeScript source" },
  { id: "image", label: "Image", summary: "inline SVG data URL" },
  { id: "audio", label: "Audio", summary: "generated WAV tone" },
  { id: "video", label: "Video", summary: "generated WebM clip" },
  { id: "pdf", label: "PDF", summary: "in-memory single-page PDF" },
  { id: "html", label: "HTML", summary: "sandboxed deterministic markup" },
  { id: "hex", label: "Hex / Binary", summary: "host supplied range fetcher" },
  { id: "book", label: "Book", summary: "local chapter reader" },
  { id: "epub", label: "EPUB", summary: "stub parser and local fetcher" },
];

export function ViewerDemoPanel() {
  const [activeId, setActiveId] = useState<ViewerDemoId>("text");
  const audioUrl = useBlobUrl(createDemoWavBlob);
  const pdfUrl = useBlobUrl(createDemoPdfBlob);
  const video = useGeneratedVideoUrl();
  const activeTab = VIEWER_DEMO_TABS.find((tab) => tab.id === activeId);
  const parseEpub = useCallback(parseDemoEpub, []);
  const fetchEpub = useCallback(fetchDemoEpub, []);
  const fetchRange = useCallback(fetchDemoHexRange, []);
  const renderedViewer = useMemo(
    () =>
      renderViewer({
        activeId,
        audioUrl,
        fetchEpub,
        fetchRange,
        parseEpub,
        pdfUrl,
        video,
      }),
    [activeId, audioUrl, fetchEpub, fetchRange, parseEpub, pdfUrl, video],
  );

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm leading-relaxed opacity-80">
        Full-feature viewer panel for plugin authors. Every viewer below is
        imported from <code>@tokimo/sdk/viewers</code> and uses local sample
        content so smoke tests never depend on remote assets.
      </p>
      <div
        className="grid gap-2 md:grid-cols-3 xl:grid-cols-5"
        data-testid="viewer-demo-tabs"
      >
        {VIEWER_DEMO_TABS.map((tab) => {
          const active = tab.id === activeId;
          return (
            <button
              key={tab.id}
              type="button"
              data-testid={`viewer-demo-tab-${tab.id}`}
              data-viewer-demo={tab.id}
              aria-pressed={active}
              onClick={() => setActiveId(tab.id)}
              className={`cursor-pointer rounded-xl border px-3 py-2 text-left transition ${
                active
                  ? "border-[var(--accent)] bg-[var(--accent-subtle)] text-[var(--accent)]"
                  : "border-black/10 bg-black/[0.03] hover:bg-black/[0.06] dark:border-white/10 dark:bg-white/[0.04] dark:hover:bg-white/[0.08]"
              }`}
            >
              <span className="block text-xs font-semibold">{tab.label}</span>
              <span className="block text-[10px] opacity-70">
                {tab.summary}
              </span>
            </button>
          );
        })}
      </div>
      <div
        data-testid="viewer-demo-panel"
        data-viewer-demo-active={activeId}
        className="flex flex-col gap-2"
      >
        <div className="flex items-baseline justify-between gap-3">
          <div>
            <div className="text-sm font-semibold">{activeTab?.label}</div>
            <div className="text-xs opacity-60">{activeTab?.summary}</div>
          </div>
          <code className="rounded bg-black/[0.06] px-2 py-1 text-[10px] dark:bg-white/[0.06]">
            data-viewer-demo-active="{activeId}"
          </code>
        </div>
        {renderedViewer}
      </div>
    </div>
  );
}

function renderViewer({
  activeId,
  audioUrl,
  fetchEpub,
  fetchRange,
  parseEpub,
  pdfUrl,
  video,
}: {
  activeId: ViewerDemoId;
  audioUrl: string | null;
  fetchEpub: (fileUrl: string) => Promise<ArrayBuffer>;
  fetchRange: (
    fileUrl: string,
    range: string,
    signal?: AbortSignal,
  ) => Promise<Response>;
  parseEpub: (buffer: ArrayBuffer) => Promise<EpubBook>;
  pdfUrl: string | null;
  video: ReturnType<typeof useGeneratedVideoUrl>;
}) {
  switch (activeId) {
    case "text":
      return (
        <ViewerHost id="text">
          <MonacoTextEditor
            fileName="viewer-demo.ts"
            content={TEXT_SAMPLE}
            language="typescript"
            readOnly
            className="h-[420px]"
          />
        </ViewerHost>
      );
    case "image":
      return (
        <ViewerHost id="image">
          <ImagePreview src={IMAGE_DATA_URL} alt="Tokimo viewer demo" />
        </ViewerHost>
      );
    case "audio":
      return (
        <ViewerHost
          id="audio"
          className="grid min-h-[260px] place-items-center p-6"
        >
          {audioUrl ? (
            <AudioPlayer
              src={audioUrl}
              fileName="tokimo-generated-tone.wav"
              id="helloworld-viewer-demo-audio"
            />
          ) : (
            <ViewerStatus label="Preparing generated WAV sample…" />
          )}
        </ViewerHost>
      );
    case "video":
      return (
        <ViewerHost id="video">
          {video.status === "ready" && video.url ? (
            <VideoPreview src={video.url} className="h-[420px]" />
          ) : (
            <ViewerStatus
              label={
                video.status === "error"
                  ? "Video generation is unavailable in this browser."
                  : "Generating deterministic WebM sample…"
              }
              detail={video.error}
              testId="viewer-demo-video-status"
            />
          )}
        </ViewerHost>
      );
    case "pdf":
      return (
        <ViewerHost id="pdf">
          {pdfUrl ? (
            <PdfEmbed
              src={pdfUrl}
              title="Tokimo demo PDF"
              className="h-[420px]"
            />
          ) : (
            <ViewerStatus label="Preparing generated PDF sample…" />
          )}
        </ViewerHost>
      );
    case "html":
      return (
        <ViewerHost id="html">
          <HtmlPreview html={HTML_SAMPLE} />
        </ViewerHost>
      );
    case "hex":
      return (
        <ViewerHost id="hex">
          <HexViewer
            fileUrl="tokimo-demo://helloworld/sample.bin"
            fileName="sample.bin"
            fetchRange={fetchRange}
          />
        </ViewerHost>
      );
    case "book":
      return (
        <ViewerHost id="book">
          <BookViewer
            bookId="tokimo-demo-book"
            initialChapterId="chapter-1"
            fetchChapter={fetchDemoChapter}
            isActive
          />
        </ViewerHost>
      );
    case "epub":
      return (
        <ViewerHost id="epub">
          <EpubViewer
            fileUrl="tokimo-demo://helloworld/sample.epub"
            fetchBook={fetchEpub}
            parseBook={parseEpub}
            isActive
          />
        </ViewerHost>
      );
  }
}

function ViewerHost({
  id,
  children,
  className = "",
}: {
  id: ViewerDemoId;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      data-testid={`viewer-demo-host-${id}`}
      data-viewer-demo-host={id}
      className={`min-h-[420px] overflow-hidden rounded-xl border border-black/10 bg-white/70 dark:border-white/10 dark:bg-black/30 ${className}`}
    >
      {children}
    </div>
  );
}

function ViewerStatus({
  label,
  detail,
  testId = "viewer-demo-status",
}: {
  label: string;
  detail?: string | null;
  testId?: string;
}) {
  return (
    <div
      data-testid={testId}
      className="flex min-h-[260px] flex-col items-center justify-center gap-2 px-6 text-center"
    >
      <div className="text-sm font-medium">{label}</div>
      {detail && <div className="max-w-md text-xs opacity-60">{detail}</div>}
    </div>
  );
}
