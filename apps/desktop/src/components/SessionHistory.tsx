import { useState, useEffect, useCallback, useRef } from "react";
import { useSessionStore } from "../stores/sessionStore";
import { useSession } from "../hooks/useSession";
import * as commands from "../lib/tauri-commands";
import type { SessionRecord } from "../lib/tauri-commands";

function formatDate(iso: string): string {
  const d = new Date(iso);
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  const time = d.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });

  if (diffDays === 0) return `Today, ${time}`;
  if (diffDays === 1) return `Yesterday, ${time}`;
  if (diffDays < 7)
    return `${d.toLocaleDateString([], { weekday: "short" })}, ${time}`;
  return d.toLocaleDateString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDuration(created: string, ended: string | null): string {
  if (!ended) return "";
  const ms = new Date(ended).getTime() - new Date(created).getTime();
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes === 0) return `${seconds}s`;
  return `${minutes}m ${seconds}s`;
}

function StatusIndicator({ session }: { session: SessionRecord }) {
  if (session.resolved === true) {
    return (
      <span className="text-accent-green flex-shrink-0" title="Resolved">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path
            d="M3.5 7.5L5.5 9.5L10.5 4.5"
            stroke="currentColor"
            strokeWidth="1.3"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>
    );
  }
  return null;
}

function OverflowMenu({
  session,
  onResolveToggle,
  onExport,
  onDelete,
}: {
  session: SessionRecord;
  onResolveToggle: () => void;
  onExport: () => void;
  onDelete: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false);
        setConfirmDelete(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div className="relative" ref={menuRef}>
      <button
        onClick={(e) => {
          e.stopPropagation();
          setOpen(!open);
          setConfirmDelete(false);
        }}
        className="w-6 h-6 rounded flex items-center justify-center text-text-muted hover:text-text-primary hover:bg-bg-tertiary transition-colors cursor-pointer"
      >
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <circle cx="2" cy="6" r="1.2" fill="currentColor" />
          <circle cx="6" cy="6" r="1.2" fill="currentColor" />
          <circle cx="10" cy="6" r="1.2" fill="currentColor" />
        </svg>
      </button>

      {open && (
        <div className="absolute right-0 top-full mt-1 w-40 bg-bg-secondary border border-border-primary rounded-lg shadow-xl z-50 py-1 overflow-hidden">
          {/* Mark resolved (only when not already resolved) */}
          {session.resolved !== true && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onResolveToggle();
                setOpen(false);
              }}
              className="w-full px-3 py-1.5 text-left text-[11px] text-text-secondary hover:bg-bg-tertiary transition-colors cursor-pointer"
            >
              Mark resolved
            </button>
          )}

          {/* Export */}
          <button
            onClick={(e) => {
              e.stopPropagation();
              onExport();
              setOpen(false);
            }}
            className="w-full px-3 py-1.5 text-left text-[11px] text-text-secondary hover:bg-bg-tertiary transition-colors cursor-pointer"
          >
            Export
          </button>

          {/* Delete */}
          <div className="border-t border-border-primary mt-1 pt-1">
            {confirmDelete ? (
              <div className="flex items-center gap-2 px-3 py-1.5">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete();
                    setOpen(false);
                    setConfirmDelete(false);
                  }}
                  className="text-[11px] text-accent-red font-medium cursor-pointer hover:underline"
                >
                  Confirm
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setConfirmDelete(false);
                  }}
                  className="text-[11px] text-text-muted cursor-pointer hover:underline"
                >
                  Cancel
                </button>
              </div>
            ) : (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setConfirmDelete(true);
                }}
                className="w-full px-3 py-1.5 text-left text-[11px] text-accent-red hover:bg-bg-tertiary transition-colors cursor-pointer"
              >
                Delete
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function SessionItem({
  session,
  onSelect,
  onExport,
  onDelete,
  onViewActions,
  onResolveToggle,
}: {
  session: SessionRecord;
  onSelect: (sessionId: string) => void;
  onExport: (sessionId: string, title: string) => void;
  onDelete: (sessionId: string) => void;
  onViewActions: (sessionId: string) => void;
  onResolveToggle: (sessionId: string, resolved: boolean) => void;
}) {
  const duration = formatDuration(session.created_at, session.ended_at);

  return (
    <div className="border-b border-border-primary last:border-b-0">
      <div
        role="button"
        tabIndex={0}
        onClick={() => onSelect(session.id)}
        onKeyDown={(e) => { if (e.key === "Enter") onSelect(session.id); }}
        className="w-full px-4 py-3 text-left hover:bg-bg-tertiary/50 transition-colors cursor-pointer"
      >
        {/* Row 1: Title + status indicator */}
        <div className="flex items-center gap-2 min-w-0">
          <p className="text-sm text-text-primary leading-snug truncate flex-1 min-w-0">
            {session.title || "Untitled session"}
          </p>
          <StatusIndicator session={session} />
        </div>

        {/* Row 2: Date, duration, actions count, overflow menu */}
        <div className="flex items-center gap-2 mt-1.5">
          <span className="text-[10px] text-text-muted">
            {formatDate(session.created_at)}
          </span>
          {duration && (
            <>
              <span className="text-[10px] text-text-muted/40">·</span>
              <span className="text-[10px] text-text-muted">{duration}</span>
            </>
          )}
          {session.change_count > 0 && (
            <>
              <span className="text-[10px] text-text-muted/40">·</span>
              <span
                onClick={(e) => {
                  e.stopPropagation();
                  onViewActions(session.id);
                }}
                className="text-[10px] text-accent-purple hover:text-accent-purple/80 hover:underline cursor-pointer"
              >
                {session.change_count} action
                {session.change_count !== 1 ? "s" : ""}
              </span>
            </>
          )}
          <span className="ml-auto">
            <OverflowMenu
              session={session}
              onResolveToggle={() =>
                onResolveToggle(session.id, session.resolved !== true)
              }
              onExport={() =>
                onExport(session.id, session.title || "session")
              }
              onDelete={() => onDelete(session.id)}
            />
          </span>
        </div>
      </div>
    </div>
  );
}

export function SessionHistory() {
  const historyOpen = useSessionStore((s) => s.historyOpen);
  const setHistoryOpen = useSessionStore((s) => s.setHistoryOpen);
  const pastSessions = useSessionStore((s) => s.pastSessions);
  const setPastSessions = useSessionStore((s) => s.setPastSessions);
  const setChanges = useSessionStore((s) => s.setChanges);
  const setChangeLogOpen = useSessionStore((s) => s.setChangeLogOpen);
  const { switchToProblem } = useSession();

  const loadSessions = useCallback(async () => {
    try {
      const sessions = await commands.listSessions();
      setPastSessions(sessions);
    } catch (err) {
      console.error("Failed to load session history:", err);
    }
  }, [setPastSessions]);

  const handleExport = useCallback(
    async (sessionId: string, title: string) => {
      try {
        const markdown = await commands.exportSession(sessionId);
        const blob = new Blob([markdown], { type: "text/markdown" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = `${title.replace(/[^a-zA-Z0-9 ]/g, "").replace(/\s+/g, "-").toLowerCase()}.md`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      } catch (err) {
        console.error("Failed to export session:", err);
      }
    },
    [],
  );

  const handleDelete = useCallback(
    async (sessionId: string) => {
      try {
        await commands.deleteSession(sessionId);
        setPastSessions(pastSessions.filter((s) => s.id !== sessionId));
      } catch (err) {
        console.error("Failed to delete session:", err);
      }
    },
    [pastSessions, setPastSessions],
  );

  const handleResolveToggle = useCallback(
    async (sessionId: string, resolved: boolean) => {
      try {
        await commands.markResolved(sessionId, resolved);
        setPastSessions(
          pastSessions.map((s) =>
            s.id === sessionId ? { ...s, resolved } : s,
          ),
        );
      } catch (err) {
        console.error("Failed to update session:", err);
      }
    },
    [pastSessions, setPastSessions],
  );

  const handleSelectSession = useCallback(
    async (sessionId: string) => {
      await switchToProblem(sessionId);
      setHistoryOpen(false);
    },
    [switchToProblem, setHistoryOpen],
  );

  const handleViewActions = useCallback(
    async (sessionId: string) => {
      try {
        const changes = await commands.getChanges(sessionId);
        setChanges(changes);
        setChangeLogOpen(true);
      } catch (err) {
        console.error("Failed to load actions:", err);
      }
    },
    [setChanges, setChangeLogOpen],
  );

  useEffect(() => {
    if (historyOpen) {
      loadSessions();
    }
  }, [historyOpen, loadSessions]);

  if (!historyOpen) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-30 bg-black/20"
        onClick={() => setHistoryOpen(false)}
      />

      {/* Slide-out panel */}
      <div className="fixed top-0 right-0 bottom-0 z-40 w-80 bg-bg-secondary border-l border-border-primary shadow-2xl flex flex-col animate-slide-in-right">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border-primary">
          <h2 className="text-sm font-semibold text-text-primary">
            Session History
          </h2>
          <button
            onClick={() => setHistoryOpen(false)}
            className="w-7 h-7 rounded-md flex items-center justify-center text-text-muted hover:text-text-primary hover:bg-bg-tertiary transition-colors cursor-pointer"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M3 3L11 11M11 3L3 11"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </div>

        {/* Sessions list */}
        <div className="flex-1 overflow-y-auto">
          {pastSessions.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full text-text-muted px-4">
              <svg
                width="32"
                height="32"
                viewBox="0 0 32 32"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
                className="mb-3 opacity-50"
              >
                <path
                  d="M16 6V16L22 22"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                />
                <circle
                  cx="16"
                  cy="16"
                  r="12"
                  stroke="currentColor"
                  strokeWidth="1.5"
                />
              </svg>
              <p className="text-xs text-center">
                No past sessions yet.
                <br />
                Sessions will appear here as you use the app.
              </p>
            </div>
          ) : (
            <div>
              {pastSessions.map((session) => (
                <SessionItem
                  key={session.id}
                  session={session}
                  onSelect={handleSelectSession}
                  onExport={handleExport}
                  onDelete={handleDelete}
                  onViewActions={handleViewActions}
                  onResolveToggle={handleResolveToggle}
                />
              ))}
            </div>
          )}
        </div>

        {/* Footer summary */}
        {pastSessions.length > 0 && (
          <div className="px-4 py-2.5 border-t border-border-primary">
            <p className="text-[10px] text-text-muted">
              {pastSessions.length} session{pastSessions.length !== 1 ? "s" : ""} total
            </p>
          </div>
        )}
      </div>
    </>
  );
}
