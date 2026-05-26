import { type ShellJobEvent, useJobEvents } from "@tokimo/sdk";
import { Button, Card, CircularProgress } from "@tokimo/ui";
import { type ChangeEvent, useCallback, useState } from "react";
import { ButtonRow, fmt, SERVICE, Section, Snapshot } from "./shared";

const BULK_JOB_TYPE = "helloworld_bulk_import";
const LONG_JOB_TYPE = "helloworld_long_running";
type DemoJobKind = "bulk" | "long";
type DemoJobType = typeof BULK_JOB_TYPE | typeof LONG_JOB_TYPE;

interface DemoJobState {
  jobId: string | null;
  status: string;
  progress: number;
  current: number | null;
  total: number | null;
  label: string | null;
  error: string | null;
  lastEvent: unknown;
}

interface ParsedJobEvent {
  jobId: string;
  jobType: DemoJobType;
  status: string;
  progress: number;
  current: number | null;
  total: number | null;
  label: string | null;
  error: string | null;
  raw: unknown;
}

const INITIAL_JOB_STATE: DemoJobState = {
  jobId: null,
  status: "idle",
  progress: 0,
  current: null,
  total: null,
  label: null,
  error: null,
  lastEvent: null,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function stringField(record: Record<string, unknown> | null, key: string) {
  const value = record?.[key];
  return typeof value === "string" && value.length > 0 ? value : null;
}

function numberField(record: Record<string, unknown> | null, key: string) {
  const value = record?.[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function getJobRecord(event: ShellJobEvent): Record<string, unknown> | null {
  if (isRecord(event.job)) return event.job;
  if (!isRecord(event.data)) return null;
  if (isRecord(event.data.job)) return event.data.job;
  return event.data;
}

function getJobType(job: Record<string, unknown>): DemoJobType | null {
  const value =
    stringField(job, "type") ??
    stringField(job, "jobType") ??
    stringField(job, "job_type") ??
    stringField(job, "kind");
  return value === BULK_JOB_TYPE || value === LONG_JOB_TYPE ? value : null;
}

function getProgressData(job: Record<string, unknown>) {
  const data = isRecord(job.data) ? job.data : null;
  return isRecord(data?.progress) ? data.progress : null;
}

function clampProgress(value: number) {
  return Math.max(0, Math.min(100, Math.round(value)));
}

function parseJobEvent(event: ShellJobEvent): ParsedJobEvent | null {
  if (event.type !== "job_update" && event.type !== "external_job_update") {
    return null;
  }
  const job = getJobRecord(event);
  if (!job) return null;

  const jobId = stringField(job, "id");
  const jobType = getJobType(job);
  if (!jobId || !jobType) return null;

  const progressData = getProgressData(job);
  const current = numberField(progressData, "current");
  const total = numberField(progressData, "total");
  const richProgress =
    current !== null && total !== null && total > 0
      ? (current / total) * 100
      : null;
  const rawProgress = numberField(job, "progress") ?? richProgress ?? 0;

  return {
    jobId,
    jobType,
    status: stringField(job, "status") ?? "unknown",
    progress: clampProgress(rawProgress),
    current,
    total,
    label: stringField(progressData, "label"),
    error: stringField(job, "error"),
    raw: event,
  };
}

async function startJob(jobType: DemoJobType, params: Record<string, number>) {
  const res = await fetch(`/api/apps/${SERVICE}/jobs/start`, {
    method: "POST",
    credentials: "include",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ type: jobType, params }),
  });
  if (!res.ok) throw new Error(`${res.status} ${await res.text()}`);
  const body: unknown = await res.json();
  if (!isRecord(body)) throw new Error("Invalid start job response");
  const jobId = stringField(body, "jobId") ?? stringField(body, "job_id");
  if (!jobId) throw new Error("Missing job id in start job response");
  return jobId;
}

function updateFromParsed(parsed: ParsedJobEvent) {
  return (prev: DemoJobState): DemoJobState => ({
    ...prev,
    jobId: parsed.jobId,
    status: parsed.status,
    progress: parsed.progress,
    current: parsed.current,
    total: parsed.total,
    label: parsed.label,
    error: parsed.error,
    lastEvent: parsed.raw,
  });
}

function JobNumberInput({
  label,
  min,
  max,
  value,
  onChange,
}: {
  label: string;
  min: number;
  max: number;
  value: number;
  onChange: (value: number) => void;
}) {
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    const next = Number(event.target.value);
    if (!Number.isFinite(next)) return;
    onChange(Math.max(min, Math.min(max, Math.round(next))));
  };

  return (
    <label className="flex items-center gap-2 text-sm">
      <span className="opacity-70">{label}</span>
      <input
        className="w-24 rounded border border-black/10 bg-white/70 px-2 py-1 text-sm dark:border-white/10 dark:bg-black/30"
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={handleChange}
      />
    </label>
  );
}

function JobStatusCard({
  title,
  state,
}: {
  title: string;
  state: DemoJobState;
}) {
  const isCompleted = state.status === "completed";
  const statusText = isCompleted
    ? "✅ Done"
    : state.status === "failed"
      ? "✗ failed"
      : state.status;
  const progressLabel =
    state.current !== null && state.total !== null
      ? `${state.current}/${state.total}`
      : `${state.progress}%`;

  return (
    <Card className="flex flex-col gap-3 p-3">
      <div className="flex items-center gap-3">
        <CircularProgress value={state.progress} size={64} strokeWidth={6}>
          <span className="text-xs font-medium">{state.progress}%</span>
        </CircularProgress>
        <div className="min-w-0 flex-1">
          <div className="text-sm font-medium">{title}</div>
          <div className="truncate text-xs opacity-60">
            {state.jobId ?? "No job yet"}
          </div>
          <div
            className={
              isCompleted
                ? "text-xs font-medium text-green-600 dark:text-green-400"
                : "text-xs opacity-70"
            }
          >
            {statusText} · {progressLabel}
          </div>
          <div className="truncate text-xs opacity-80">
            {state.label ?? "Idle — no event received yet"}
          </div>
        </div>
      </div>
      {state.error && <div className="text-sm text-red-500">{state.error}</div>}
      <details className="text-xs">
        <summary className="cursor-pointer select-none opacity-70">
          jobId / status / raw last event JSON
        </summary>
        <Snapshot>
          {fmt({
            jobId: state.jobId,
            status: state.status,
            rawLastEvent: state.lastEvent,
          })}
        </Snapshot>
      </details>
    </Card>
  );
}

function useHelloworldJobs(kind: DemoJobKind) {
  const [state, setState] = useState<DemoJobState>(INITIAL_JOB_STATE);
  const [startError, setStartError] = useState<string | null>(null);
  const jobType = kind === "bulk" ? BULK_JOB_TYPE : LONG_JOB_TYPE;

  const applyEvent = useCallback(
    (event: ShellJobEvent) => {
      const parsed = parseJobEvent(event);
      if (!parsed || parsed.jobType !== jobType) return;
      setState((prev) => {
        if (prev.jobId !== parsed.jobId) return prev;
        return updateFromParsed(parsed)(prev);
      });
    },
    [jobType],
  );

  useJobEvents({ jobTypes: [jobType], onEvent: applyEvent });

  const start = useCallback(
    async (params: Record<string, number>) => {
      setStartError(null);
      try {
        const jobId = await startJob(jobType, params);
        setState({
          ...INITIAL_JOB_STATE,
          jobId,
          status: "queued",
          lastEvent: { jobId, jobType, params },
        });
      } catch (e) {
        setStartError(e instanceof Error ? e.message : String(e));
      }
    },
    [jobType],
  );

  return { state, start, startError };
}

export function BulkImportJobDemo() {
  const [total, setTotal] = useState(50);
  const { state, start, startError } = useHelloworldJobs("bulk");

  return (
    <Section
      desc="Starts a simulated bulk import job and updates progress from WebSocket job events only."
      code="useJobEvents({ jobTypes: ['helloworld_bulk_import'], onEvent })"
    >
      <ButtonRow>
        <JobNumberInput
          label="Items"
          min={1}
          max={500}
          value={total}
          onChange={setTotal}
        />
        <Button variant="primary" onClick={() => start({ total })}>
          {state.jobId ? "Run again" : "Start bulk import"}
        </Button>
      </ButtonRow>
      {startError && <div className="text-sm text-red-500">{startError}</div>}
      <JobStatusCard title="Bulk import" state={state} />
    </Section>
  );
}

export function LongRunningJobDemo() {
  const [durationSecs, setDurationSecs] = useState(30);
  const { state, start, startError } = useHelloworldJobs("long");

  return (
    <Section
      desc="Starts a simulated long-running job and renders progress from WebSocket job_update payloads only."
      code="useJobEvents({ jobTypes: ['helloworld_long_running'], onEvent })"
    >
      <ButtonRow>
        <JobNumberInput
          label="Duration (s)"
          min={1}
          max={300}
          value={durationSecs}
          onChange={setDurationSecs}
        />
        <Button
          variant="primary"
          onClick={() => start({ duration_secs: durationSecs })}
        >
          {state.jobId ? "Run again" : "Start long running"}
        </Button>
      </ButtonRow>
      {startError && <div className="text-sm text-red-500">{startError}</div>}
      <JobStatusCard title="Long running" state={state} />
    </Section>
  );
}
