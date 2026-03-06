import { useSessionStore } from "../stores/sessionStore";
import { NoahIcon } from "./NoahIcon";

interface SessionBarProps {
  session: {
    startNewProblem: () => Promise<void>;
  };
}

export function SessionBar({ session }: SessionBarProps) {
  const toggleHistory = useSessionStore((s) => s.toggleHistory);
  const historyOpen = useSessionStore((s) => s.historyOpen);
  const toggleKnowledge = useSessionStore((s) => s.toggleKnowledge);
  const knowledgeOpen = useSessionStore((s) => s.knowledgeOpen);
  const toggleSettings = useSessionStore((s) => s.toggleSettings);
  const settingsOpen = useSessionStore((s) => s.settingsOpen);

  return (
    <header
      className="flex items-center justify-between px-4 py-2.5 bg-bg-secondary border-b border-border-primary select-none"
      data-tauri-drag-region=""
    >
      {/* Left: Brand + New conversation */}
      <div className="flex items-center gap-3" data-tauri-drag-region="">
        <div className="flex items-center gap-2">
          <NoahIcon className="w-7 h-7 rounded-lg" alt="Noah" />
          <span className="text-sm font-semibold text-text-primary">
            Noah
          </span>
        </div>
        <button
          onClick={session.startNewProblem}
          className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium text-accent-blue hover:bg-accent-blue/10 transition-colors cursor-pointer"
        >
          <svg width="12" height="12" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M7 3V11M3 7H11" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
          </svg>
          New
        </button>
      </div>

      {/* Right: Panel toggles */}
      <div className="flex items-center gap-0.5">
        <PanelButton
          label="History"
          active={historyOpen}
          onClick={toggleHistory}
          icon={<svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M7 3.5V7L9.5 9.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" /><circle cx="7" cy="7" r="5.5" stroke="currentColor" strokeWidth="1.2" /></svg>}
        />
        <PanelButton
          label="Knowledge"
          active={knowledgeOpen}
          onClick={toggleKnowledge}
          icon={<svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M2 2.5C2 2.5 3.5 1.5 7 1.5C10.5 1.5 12 2.5 12 2.5V11.5C12 11.5 10.5 10.5 7 10.5C3.5 10.5 2 11.5 2 11.5V2.5Z" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round" /><path d="M7 1.5V10.5" stroke="currentColor" strokeWidth="1.1" /></svg>}
        />
        <PanelButton
          label=""
          active={settingsOpen}
          onClick={toggleSettings}
          icon={<svg width="14" height="14" viewBox="0 0 14 14" fill="none"><path d="M5.7 1.5H8.3L8.8 3.1L10.3 3.9L11.9 3.4L13.2 5.6L11.9 6.8V7.2L13.2 8.4L11.9 10.6L10.3 10.1L8.8 10.9L8.3 12.5H5.7L5.2 10.9L3.7 10.1L2.1 10.6L0.8 8.4L2.1 7.2V6.8L0.8 5.6L2.1 3.4L3.7 3.9L5.2 3.1L5.7 1.5Z" stroke="currentColor" strokeWidth="1.1" strokeLinejoin="round" /><circle cx="7" cy="7" r="1.8" stroke="currentColor" strokeWidth="1.1" /></svg>}
        />
      </div>
    </header>
  );
}

function PanelButton({
  label,
  active,
  onClick,
  icon,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`
        flex items-center gap-1.5 px-2 py-1.5 rounded-lg text-xs
        transition-colors duration-150 cursor-pointer
        ${
          active
            ? "bg-accent-blue/15 text-accent-blue"
            : "text-text-muted hover:text-text-secondary hover:bg-bg-tertiary/50"
        }
      `}
    >
      {icon}
      {label && <span>{label}</span>}
    </button>
  );
}
