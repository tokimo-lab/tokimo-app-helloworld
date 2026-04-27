import type { EpubBook } from "@tokimo/sdk/viewers";
import { useEffect, useState } from "react";

export const TEXT_SAMPLE = `import { defineApp } from "@tokimo/sdk";
import { MonacoTextEditor } from "@tokimo/sdk/viewers";

export default defineApp({
  id: "viewer-demo",
  manifest: {
    appName: "Viewer Demo",
    windowType: "viewer-demo",
  },
});`;

const IMAGE_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="720" height="420" viewBox="0 0 720 420">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#10b981"/>
      <stop offset="1" stop-color="#2563eb"/>
    </linearGradient>
  </defs>
  <rect width="720" height="420" rx="28" fill="url(#g)"/>
  <circle cx="580" cy="90" r="64" fill="rgba(255,255,255,0.28)"/>
  <path d="M78 315 236 172l118 91 92-70 196 122H78Z" fill="rgba(255,255,255,0.72)"/>
  <text x="72" y="92" fill="white" font-size="42" font-family="Inter,Arial,sans-serif" font-weight="700">Tokimo Viewer Demo</text>
  <text x="74" y="134" fill="rgba(255,255,255,0.78)" font-size="20" font-family="Inter,Arial,sans-serif">SVG data URL · no remote asset</text>
</svg>`;

export const IMAGE_DATA_URL = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
  IMAGE_SVG,
)}`;

export const HTML_SAMPLE = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <style>
      body {
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        font-family: Inter, system-ui, sans-serif;
        color: #0f172a;
        background: linear-gradient(135deg, #ecfdf5, #dbeafe);
      }
      main {
        width: min(560px, calc(100vw - 48px));
        border: 1px solid rgba(15, 23, 42, 0.12);
        border-radius: 24px;
        padding: 28px;
        background: rgba(255, 255, 255, 0.78);
        box-shadow: 0 24px 80px rgba(15, 23, 42, 0.16);
      }
      code { color: #047857; }
    </style>
  </head>
  <body>
    <main data-testid="viewer-demo-html-document">
      <h1>Sandboxed HTML Preview</h1>
      <p>This deterministic document is passed as a string to <code>HtmlPreview</code>.</p>
      <ul>
        <li>No network request</li>
        <li>Stable text for smoke tests</li>
        <li>Runs inside the viewer iframe sandbox</li>
      </ul>
    </main>
  </body>
</html>`;

const HEX_BYTES = new Uint8Array(
  Array.from({ length: 1536 }, (_value, index) => (index * 17 + 41) % 256),
);

const BOOK_CHAPTERS = [
  {
    id: "chapter-1",
    title: "Chapter 1 · A Window Opens",
    content:
      "Tokimo starts with a desktop, then opens a focused reader panel. This chapter is short, deterministic, and loaded through fetchChapter.",
  },
  {
    id: "chapter-2",
    title: "Chapter 2 · Stable Automation",
    content:
      "Every tab and host in this demo exposes data-testid and data-viewer-demo hooks so smoke tests can exercise the full viewer surface.",
  },
  {
    id: "chapter-3",
    title: "Chapter 3 · SDK Imports",
    content:
      "Plugin authors import reusable viewers from @tokimo/sdk/viewers instead of private web application paths.",
  },
] as const;

const EPUB_CHAPTER_HTML = [
  `<article><h1>EPUB Demo · Cover</h1><p>This EPUB viewer uses a local fetchBook callback and a stub parseBook implementation.</p></article>`,
  `<article><h1>Automation Contract</h1><p>The active host is <strong>viewer-demo-host-epub</strong>, and no external EPUB file is requested.</p></article>`,
];

const VIDEO_MIME_TYPES = [
  "video/webm;codecs=vp9",
  "video/webm;codecs=vp8",
  "video/webm",
];

export function useBlobUrl(createBlob: () => Blob): string | null {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    const nextUrl = URL.createObjectURL(createBlob());
    setUrl(nextUrl);
    return () => URL.revokeObjectURL(nextUrl);
  }, [createBlob]);

  return url;
}

export function createDemoWavBlob(): Blob {
  const sampleRate = 8000;
  const durationSeconds = 0.75;
  const bytesPerSample = 2;
  const sampleCount = Math.floor(sampleRate * durationSeconds);
  const dataSize = sampleCount * bytesPerSample;
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);

  writeAscii(view, 0, "RIFF");
  view.setUint32(4, 36 + dataSize, true);
  writeAscii(view, 8, "WAVE");
  writeAscii(view, 12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * bytesPerSample, true);
  view.setUint16(32, bytesPerSample, true);
  view.setUint16(34, 16, true);
  writeAscii(view, 36, "data");
  view.setUint32(40, dataSize, true);

  for (let index = 0; index < sampleCount; index += 1) {
    const t = index / sampleRate;
    const envelope = Math.sin((Math.PI * index) / sampleCount);
    const sample = Math.sin(2 * Math.PI * 440 * t) * envelope * 0.36;
    view.setInt16(44 + index * bytesPerSample, sample * 0x7fff, true);
  }

  return new Blob([buffer], { type: "audio/wav" });
}

function writeAscii(view: DataView, offset: number, value: string): void {
  for (let index = 0; index < value.length; index += 1) {
    view.setUint8(offset + index, value.charCodeAt(index));
  }
}

export function createDemoPdfBlob(): Blob {
  const stream = [
    "BT",
    "/F1 24 Tf",
    "72 720 Td",
    "(Tokimo viewer demo PDF) Tj",
    "/F1 12 Tf",
    "0 -36 Td",
    "(Generated in memory for smoke and e2e regression.) Tj",
    "ET",
  ].join("\n");
  const streamLength = new TextEncoder().encode(stream).byteLength;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    `<< /Length ${streamLength} >>\nstream\n${stream}\nendstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  let pdf = "%PDF-1.4\n";
  const offsets: number[] = [];
  const encoder = new TextEncoder();

  for (let index = 0; index < objects.length; index += 1) {
    offsets.push(encoder.encode(pdf).byteLength);
    pdf += `${index + 1} 0 obj\n${objects[index]}\nendobj\n`;
  }

  const xrefOffset = encoder.encode(pdf).byteLength;
  pdf += `xref\n0 ${objects.length + 1}\n`;
  pdf += "0000000000 65535 f \n";
  for (const offset of offsets) {
    pdf += `${offset.toString().padStart(10, "0")} 00000 n \n`;
  }
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\n`;
  pdf += `startxref\n${xrefOffset}\n%%EOF\n`;

  return new Blob([pdf], { type: "application/pdf" });
}

function drawVideoFrame(
  context: CanvasRenderingContext2D,
  frame: number,
): void {
  const { canvas } = context;
  const progress = frame / 23;
  context.fillStyle = "#020617";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = "#10b981";
  context.fillRect(28, 32, 92 + progress * 148, 28);
  context.fillStyle = "#2563eb";
  context.beginPath();
  context.arc(98 + progress * 164, 136, 42, 0, Math.PI * 2);
  context.fill();
  context.fillStyle = "#ffffff";
  context.font = "700 24px sans-serif";
  context.fillText("Tokimo Video Demo", 28, 214);
  context.font = "14px sans-serif";
  context.fillText(
    `generated frame ${frame.toString().padStart(2, "0")}`,
    28,
    240,
  );
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

export function useGeneratedVideoUrl(): {
  url: string | null;
  status: "loading" | "ready" | "error";
  error: string | null;
} {
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;
    const canvas = document.createElement("canvas");
    canvas.width = 360;
    canvas.height = 260;
    const context = canvas.getContext("2d");
    const stream =
      typeof canvas.captureStream === "function"
        ? canvas.captureStream(12)
        : null;

    async function record(): Promise<void> {
      if (!context) throw new Error("Canvas 2D context is unavailable");
      if (!stream) throw new Error("Canvas captureStream is unavailable");
      if (typeof MediaRecorder === "undefined") {
        throw new Error("MediaRecorder is unavailable");
      }
      const mimeType = VIDEO_MIME_TYPES.find((type) =>
        MediaRecorder.isTypeSupported(type),
      );
      if (!mimeType) throw new Error("No supported WebM MediaRecorder codec");

      const chunks: BlobPart[] = [];
      const recorder = new MediaRecorder(stream, { mimeType });
      const done = new Promise<Blob>((resolve, reject) => {
        recorder.ondataavailable = (event) => {
          if (event.data.size > 0) chunks.push(event.data);
        };
        recorder.onerror = () => reject(new Error("MediaRecorder failed"));
        recorder.onstop = () => resolve(new Blob(chunks, { type: mimeType }));
      });

      recorder.start();
      for (let frame = 0; frame < 24; frame += 1) {
        if (cancelled) break;
        drawVideoFrame(context, frame);
        await wait(34);
      }
      if (recorder.state !== "inactive") recorder.stop();
      const blob = await done;
      if (cancelled) return;
      objectUrl = URL.createObjectURL(blob);
      setUrl(objectUrl);
    }

    record()
      .catch((err: unknown) => {
        console.error("[helloworld] failed to generate demo video:", err);
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      })
      .finally(() => {
        stream?.getTracks().forEach((track) => {
          track.stop();
        });
      });

    return () => {
      cancelled = true;
      stream?.getTracks().forEach((track) => {
        track.stop();
      });
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, []);

  return {
    url,
    status: error ? "error" : url ? "ready" : "loading",
    error,
  };
}

function uint8ToArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  return buffer;
}

export async function fetchDemoHexRange(
  _fileUrl: string,
  range: string,
  signal?: AbortSignal,
): Promise<Response> {
  if (signal?.aborted) throw new DOMException("Aborted", "AbortError");
  const match = /^bytes=(\d+)-(\d+)$/.exec(range);
  const start = match?.[1] ? Number(match[1]) : 0;
  const requestedEnd = match?.[2] ? Number(match[2]) : HEX_BYTES.length - 1;
  const end = Math.min(requestedEnd, HEX_BYTES.length - 1);
  const chunk = HEX_BYTES.slice(start, end + 1);
  return new Response(uint8ToArrayBuffer(chunk), {
    status: 206,
    headers: {
      "Accept-Ranges": "bytes",
      "Content-Length": String(chunk.byteLength),
      "Content-Range": `bytes ${start}-${end}/${HEX_BYTES.length}`,
      "Content-Type": "application/octet-stream",
    },
  });
}

export async function fetchDemoChapter(_bookId: string, chapterId: string) {
  const index = BOOK_CHAPTERS.findIndex((chapter) => chapter.id === chapterId);
  const chapter = index >= 0 ? BOOK_CHAPTERS[index] : BOOK_CHAPTERS[0];
  return {
    id: chapter.id,
    title: chapter.title,
    chapterNumber: index >= 0 ? index + 1 : 1,
    content: chapter.content,
    prevChapterId: index > 0 ? BOOK_CHAPTERS[index - 1].id : null,
    nextChapterId:
      index >= 0 && index < BOOK_CHAPTERS.length - 1
        ? BOOK_CHAPTERS[index + 1].id
        : null,
    bookTitle: "Tokimo Demo Book",
    volumeTitle: "Viewer Samples",
  };
}

export async function fetchDemoEpub(): Promise<ArrayBuffer> {
  return uint8ToArrayBuffer(new TextEncoder().encode("tokimo-demo-epub"));
}

export async function parseDemoEpub(_buffer: ArrayBuffer): Promise<EpubBook> {
  return {
    spine: [
      { id: "cover", href: "cover.xhtml", mediaType: "application/xhtml+xml" },
      {
        id: "automation",
        href: "automation.xhtml",
        mediaType: "application/xhtml+xml",
      },
    ],
    toc: [
      { id: "cover", label: "EPUB Demo", href: "cover.xhtml", children: [] },
      {
        id: "automation",
        label: "Automation Contract",
        href: "automation.xhtml",
        children: [],
      },
    ],
    getChapterHtml: async (index) =>
      EPUB_CHAPTER_HTML[index] ?? EPUB_CHAPTER_HTML[0],
    destroy: () => undefined,
  };
}
