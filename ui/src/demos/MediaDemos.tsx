import type { AppRuntimeCtx } from "@tokimo/sdk";
import { useMediaCenter } from "@tokimo/sdk/react";
import { Button, Card, Empty } from "@tokimo/ui";
import { useEffect } from "react";
import { ButtonRow, fmt, Section, Snapshot } from "./shared";

export function MediaCenterSnapshotDemo({ ctx }: { ctx: AppRuntimeCtx }) {
  const { snapshot } = useMediaCenter(ctx);

  useEffect(() => {
    console.log("[helloworld] media.snapshot →", snapshot);
  }, [snapshot]);

  return (
    <Section
      desc="Central media center snapshot. Reactive — re-renders whenever the active provider's playback state changes. `null` = no provider playing."
      code="const { snapshot } = useMediaCenter(ctx);"
    >
      {snapshot == null && (
        <Empty description="Media center idle — start playback in another app first." />
      )}
      <Snapshot>{fmt(snapshot ?? null)}</Snapshot>
    </Section>
  );
}

export function MediaSessionDemo({ ctx }: { ctx: AppRuntimeCtx }) {
  const { snapshot, api } = useMediaCenter(ctx);
  const mediaApi = api ?? ctx.shell.media;

  if (snapshot == null) {
    return (
      <Section
        desc="Compact controls for the active media session."
        code="const { snapshot, api } = useMediaCenter(ctx);"
      >
        <Empty description="No active media source — start playback in another app first." />
      </Section>
    );
  }

  const currentTrack = snapshot.queue[snapshot.currentIndex] ?? null;

  return (
    <Section
      desc="Compact active source plus pause/resume and next/previous controls."
      code="const { snapshot, api } = useMediaCenter(ctx);"
    >
      <Card className="flex items-center gap-3 p-3">
        {currentTrack?.artworkUrl ? (
          <img
            alt=""
            className="h-14 w-14 rounded-md object-cover"
            src={currentTrack.artworkUrl}
          />
        ) : (
          <div className="flex h-14 w-14 items-center justify-center rounded-md bg-black/10 text-xs opacity-60 dark:bg-white/10">
            Art
          </div>
        )}
        <div className="min-w-0 flex-1">
          <div className="text-xs opacity-60">
            {snapshot.providerId} ·{" "}
            {snapshot.isPlaying ? "isPlaying" : "paused"}
          </div>
          <div className="truncate text-sm font-medium">
            {currentTrack?.title ?? "Untitled track"}
          </div>
          <div className="truncate text-xs opacity-70">
            {currentTrack?.artist ?? "Unknown artist"}
          </div>
        </div>
      </Card>
      <ButtonRow>
        <Button size="small" onClick={() => mediaApi.previous()}>
          ⏮ prev
        </Button>
        <Button
          size="small"
          onClick={() => {
            snapshot.isPlaying ? mediaApi.pause() : mediaApi.resume();
          }}
        >
          {snapshot.isPlaying ? "⏸ pause" : "▶ play"}
        </Button>
        <Button size="small" onClick={() => mediaApi.next()}>
          ⏭ next
        </Button>
      </ButtonRow>
    </Section>
  );
}
