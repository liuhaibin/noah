import { useState, useCallback } from "react";
import * as commands from "../lib/tauri-commands";
import { NoahIcon } from "./NoahIcon";

interface SetupScreenProps {
  onComplete: () => void;
}

export function SetupScreen({ onComplete }: SetupScreenProps) {
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const handleSave = useCallback(async () => {
    const key = apiKey.trim();
    if (!key) {
      setError("Please paste your API key.");
      return;
    }
    if (!key.startsWith("sk-ant-")) {
      setError("That doesn't look like an Anthropic API key. It should start with sk-ant-");
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await commands.setApiKey(key);
      onComplete();
    } catch (err) {
      setError(`Failed to save: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSaving(false);
    }
  }, [apiKey, onComplete]);

  return (
    <div className="flex flex-col items-center justify-center h-screen bg-bg-primary px-6">
      <div className="w-full max-w-md">
        {/* Logo */}
        <div className="flex flex-col items-center mb-8">
          <NoahIcon className="w-16 h-16 rounded-2xl mb-4" alt="Noah" />
          <h1 className="text-xl font-semibold text-text-primary">
            Welcome to Noah
          </h1>
          <p className="text-sm text-text-secondary mt-2 text-center leading-relaxed">
            Noah uses Claude by Anthropic to help fix your computer.
            <br />
            You'll need an API key to get started.
          </p>
        </div>

        {/* Input */}
        <div className="space-y-4">
          <div>
            <label
              htmlFor="api-key"
              className="block text-xs text-text-secondary mb-1.5"
            >
              Anthropic API Key
            </label>
            <input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSave();
              }}
              placeholder="sk-ant-..."
              className="w-full px-4 py-2.5 rounded-xl bg-bg-input border border-border-primary text-sm text-text-primary placeholder-text-muted outline-none focus:border-border-focus transition-colors"
              autoFocus
            />
            {error && (
              <p className="text-xs text-accent-red mt-1.5">{error}</p>
            )}
          </div>

          <button
            onClick={handleSave}
            disabled={saving}
            className="w-full py-2.5 rounded-xl bg-accent-green text-white text-sm font-medium hover:bg-accent-green/80 transition-colors cursor-pointer disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save & Start"}
          </button>

          <p className="text-[11px] text-text-muted text-center leading-relaxed">
            Don't have a key?{" "}
            <a
              href="https://console.anthropic.com"
              target="_blank"
              rel="noopener noreferrer"
              className="text-accent-green hover:underline"
            >
              Get one from Anthropic
            </a>
            .
            <br />
            Your key is saved locally and never shared.
          </p>
        </div>
      </div>
    </div>
  );
}
