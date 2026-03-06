import { useState, useEffect } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export function UpdateBanner() {
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [status, setStatus] = useState<"idle" | "downloading" | "installing" | "done" | "error">("idle");
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function checkForUpdate() {
      try {
        const update = await check();
        if (!cancelled && update?.available) {
          setUpdateVersion(update.version);
        }
      } catch {
        // Silently ignore update check failures (offline, no endpoint, etc.)
      }
    }

    // Check on mount
    checkForUpdate();

    // Check every 6 hours
    const interval = setInterval(checkForUpdate, 6 * 60 * 60 * 1000);

    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  if (!updateVersion || dismissed) return null;

  const handleInstall = async () => {
    setStatus("downloading");
    try {
      const update = await check();
      if (update?.available) {
        await update.downloadAndInstall((event) => {
          if (event.event === "Finished") {
            setStatus("installing");
          }
        });
        setStatus("done");
        // Relaunch the app to apply the update
        await relaunch();
      }
    } catch (err) {
      console.error("Update failed:", err);
      setStatus("error");
      // Reset after a few seconds so user can retry
      setTimeout(() => setStatus("idle"), 5000);
    }
  };

  const buttonLabel = {
    idle: "Update now",
    downloading: "Downloading...",
    installing: "Restarting...",
    done: "Restarting...",
    error: "Update failed",
  }[status];

  return (
    <div className="flex items-center justify-between gap-3 px-4 py-2 bg-accent-blue/10 border-b border-accent-blue/20">
      <p className="text-xs text-text-primary">
        <span className="font-medium">Noah v{updateVersion}</span> is
        available.
      </p>
      <div className="flex items-center gap-2">
        {status === "idle" && (
          <button
            onClick={() => setDismissed(true)}
            className="text-[10px] text-text-muted hover:text-text-primary transition-colors cursor-pointer"
          >
            Later
          </button>
        )}
        <button
          onClick={handleInstall}
          disabled={status !== "idle"}
          className={`px-3 py-1 rounded-md text-[11px] font-medium transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed ${
            status === "error"
              ? "bg-accent-red text-white"
              : "bg-accent-blue text-white hover:bg-accent-blue/80"
          }`}
        >
          {buttonLabel}
        </button>
      </div>
    </div>
  );
}
