import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useSessionStore } from "../stores/sessionStore";
import * as commands from "../lib/tauri-commands";
import type { ScanJobRecord } from "../lib/tauri-commands";
import { useLocale } from "../i18n";

interface ScanProgressEvent {
  scan_type: string;
  display_name: string;
  status: string;
  progress_pct: number;
  progress_detail: string;
}

const DISPLAY_NAME_KEYS: Record<string, string> = {
  disk: "diagnostics.diskAnalysis",
};

function formatRelativeTime(iso: string | null): string {
  if (!iso) return "Never";
  const d = new Date(iso);
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "Just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

function statusColor(status: string): string {
  switch (status) {
    case "running":
      return "text-accent-blue";
    case "completed":
      return "text-accent-green";
    case "failed":
      return "text-accent-red";
    case "paused":
      return "text-accent-yellow";
    default:
      return "text-text-muted";
  }
}

const STATUS_LABEL_KEYS: Record<string, string> = {
  running: "diagnostics.running",
  completed: "diagnostics.complete",
  failed: "diagnostics.failed",
  paused: "diagnostics.paused",
  queued: "diagnostics.queued",
  skipped: "diagnostics.skipped",
};

function ScanJobItem({
  job,
  liveProgress,
  onTrigger,
  onPause,
  onResume,
}: {
  job: ScanJobRecord;
  liveProgress: ScanProgressEvent | null;
  onTrigger: (scanType: string) => void;
  onPause: (scanType: string) => void;
  onResume: (scanType: string) => void;
}) {
  const { t } = useLocale();
  const status = liveProgress?.status || job.status;
  const pct = liveProgress?.progress_pct ?? job.progress_pct;
  const detail = liveProgress?.progress_detail || job.progress_detail || "";
  const displayNameKey = DISPLAY_NAME_KEYS[job.scan_type];
  const displayName =
    liveProgress?.display_name ||
    (displayNameKey ? t(displayNameKey) : job.scan_type);
  const isRunning = status === "running";
  const isPaused = status === "paused";
  const hasRecentScan = (() => {
    const ts = job.completed_at || job.updated_at;
    if (!ts || status === "queued") return false;
    return Date.now() - new Date(ts).getTime() < 60 * 60 * 1000; // < 1h
  })();

  // When running, reset pct to the live value (don't carry over 100% from a prior run).
  const displayPct = isRunning ? (liveProgress?.progress_pct ?? 0) : pct;

  // Build a single status line: detail text when running, otherwise the status label.
  const statusText = isRunning
    ? detail || t("diagnostics.starting")
    : isPaused
      ? detail || t(STATUS_LABEL_KEYS.paused)
      : STATUS_LABEL_KEYS[status]
        ? t(STATUS_LABEL_KEYS[status])
        : status;

  return (
    <div className="border border-border-primary/50 rounded-lg p-4">
      <div className="flex items-center justify-between mb-2">
        <span className="text-base font-medium text-text-primary">
          {displayName}
        </span>
        <div className="flex items-center gap-2">
          {isRunning ? (
            <button
              onClick={() => onPause(job.scan_type)}
              className="text-xs px-2.5 py-1 rounded-md bg-bg-tertiary text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
            >
              {t("diagnostics.pause")}
            </button>
          ) : isPaused ? (
            <button
              onClick={() => onResume(job.scan_type)}
              className="text-xs px-2.5 py-1 rounded-md bg-bg-tertiary text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
            >
              {t("diagnostics.resume")}
            </button>
          ) : (
            <button
              onClick={() => onTrigger(job.scan_type)}
              className="text-xs px-2.5 py-1 rounded-md bg-bg-tertiary text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
            >
              {hasRecentScan ? t("diagnostics.rescan") : t("diagnostics.scanNow")}
            </button>
          )}
        </div>
      </div>

      {/* Progress bar — only show when actively running or paused */}
      {(isRunning || isPaused) && (
        <div className="w-full h-1.5 bg-bg-tertiary rounded-full overflow-hidden mb-2">
          <div
            className={`h-full rounded-full transition-all duration-500 ${
              isRunning ? "bg-accent-blue" : "bg-accent-yellow"
            }`}
            style={{ width: `${Math.max(displayPct, 2)}%` }}
          />
        </div>
      )}

      <div className="flex items-center justify-between text-xs text-text-muted">
        <span className={`truncate flex-1 mr-4 ${statusColor(status)}`}>
          {statusText}
        </span>
        <span className="flex-shrink-0">
          {job.completed_at
            ? formatRelativeTime(job.completed_at)
            : job.updated_at
              ? formatRelativeTime(job.updated_at)
              : ""}
        </span>
      </div>
    </div>
  );
}

export function DiagnosticsView() {
  const { t } = useLocale();
  const activeView = useSessionStore((s) => s.activeView);
  const [jobs, setJobs] = useState<ScanJobRecord[]>([]);
  const [liveProgress, setLiveProgress] = useState<
    Record<string, ScanProgressEvent>
  >({});

  const loadJobs = useCallback(async () => {
    try {
      const result = await commands.getScanJobs();
      setJobs(result);
    } catch (err) {
      console.error("Failed to load scan jobs:", err);
    }
  }, []);

  // Load jobs when view becomes active.
  useEffect(() => {
    if (activeView === "diagnostics") {
      loadJobs();
    }
  }, [activeView, loadJobs]);

  // Listen for live scanner-progress events.
  useEffect(() => {
    const unlisten = listen<ScanProgressEvent>("scanner-progress", (e) => {
      setLiveProgress((prev) => ({
        ...prev,
        [e.payload.scan_type]: e.payload,
      }));
      // If completed/failed, refresh the jobs list.
      if (
        e.payload.status === "completed" ||
        e.payload.status === "failed"
      ) {
        loadJobs();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadJobs]);

  const handleTrigger = useCallback(
    async (scanType: string) => {
      try {
        await commands.triggerScan(scanType);
        setLiveProgress((prev) => ({
          ...prev,
          [scanType]: {
            scan_type: scanType,
            display_name: DISPLAY_NAME_KEYS[scanType] ? t(DISPLAY_NAME_KEYS[scanType]) : scanType,
            status: "running",
            progress_pct: 0,
            progress_detail: t("diagnostics.starting"),
          },
        }));
      } catch (err) {
        console.error("Failed to trigger scan:", err);
      }
    },
    [],
  );

  const handlePause = useCallback(async (scanType: string) => {
    try {
      await commands.pauseScan(scanType);
      setLiveProgress((prev) => ({
        ...prev,
        [scanType]: {
          ...(prev[scanType] || {
            scan_type: scanType,
            display_name: DISPLAY_NAME_KEYS[scanType] ? t(DISPLAY_NAME_KEYS[scanType]) : scanType,
            progress_pct: 0,
            progress_detail: "",
          }),
          status: "paused",
        },
      }));
    } catch (err) {
      console.error("Failed to pause scan:", err);
    }
  }, []);

  const handleResume = useCallback(async (scanType: string) => {
    try {
      await commands.resumeScan(scanType);
      setLiveProgress((prev) => ({
        ...prev,
        [scanType]: {
          ...(prev[scanType] || {
            scan_type: scanType,
            display_name: DISPLAY_NAME_KEYS[scanType] ? t(DISPLAY_NAME_KEYS[scanType]) : scanType,
            progress_pct: 0,
            progress_detail: "",
          }),
          status: "running",
        },
      }));
    } catch (err) {
      console.error("Failed to resume scan:", err);
    }
  }, []);

  // Dedupe: show jobs from DB, overlayed with live data.
  // If no jobs at all, show placeholder for known scanners.
  const knownScanTypes = ["disk"];
  const displayJobs: ScanJobRecord[] =
    jobs.length > 0
      ? jobs
      : knownScanTypes.map((st) => ({
          id: st,
          scan_type: st,
          status: "queued",
          progress_pct: 0,
          progress_detail: t("diagnostics.waitingFirstScan"),
          budget_secs: null,
          started_at: null,
          updated_at: null,
          completed_at: null,
          config: null,
        }));

  // Deduplicate to latest per scan_type.
  const byType = new Map<string, ScanJobRecord>();
  for (const job of displayJobs) {
    if (!byType.has(job.scan_type)) {
      byType.set(job.scan_type, job);
    }
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl w-full mx-auto py-4 px-6">
          <div className="pb-4">
            <h1 className="text-2xl font-semibold text-text-primary">
              {t("diagnostics.title")}
            </h1>
            <p className="text-sm text-text-muted mt-1">
              {t("diagnostics.subtitle")}
            </p>
          </div>

          <div className="space-y-3">
            {Array.from(byType.values()).map((job) => (
              <ScanJobItem
                key={job.scan_type}
                job={job}
                liveProgress={liveProgress[job.scan_type] || null}
                onTrigger={handleTrigger}
                onPause={handlePause}
                onResume={handleResume}
              />
            ))}
          </div>

          <div className="mt-8 text-xs text-text-muted">
            <p>
              {t("diagnostics.scanFooter")}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
