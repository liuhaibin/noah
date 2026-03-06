// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// jsdom doesn't implement scrollIntoView — stub it for ChatPanel's useEffect
Element.prototype.scrollIntoView = vi.fn();
import type { ChangeEntry, SessionRecord } from "../lib/tauri-commands";

// ── Tauri shims ──────────────────────────────────────────────────────────────

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("../lib/tauri-commands", () => ({
  listKnowledge: vi.fn().mockResolvedValue([]),
  listSessions: vi.fn().mockResolvedValue([]),
  getChanges: vi.fn().mockResolvedValue([]),
  exportSession: vi.fn().mockResolvedValue(""),
  deleteSession: vi.fn().mockResolvedValue(undefined),
  sendMessage: vi.fn().mockResolvedValue(""),
  cancelProcessing: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../hooks/useAgent", () => ({
  useAgent: () => ({
    sendMessage: vi.fn(),
    sendConfirmation: vi.fn(),
    cancelProcessing: vi.fn(),
    isProcessing: false,
  }),
}));
vi.mock("../hooks/useSession", () => ({
  useSession: () => ({
    switchToProblem: vi.fn(),
    sessionId: "s1",
    isActive: true,
  }),
}));

// ── Stores ───────────────────────────────────────────────────────────────────

import { useSessionStore } from "../stores/sessionStore";
import { useChatStore } from "../stores/chatStore";
import * as commands from "../lib/tauri-commands";

// ── Components ───────────────────────────────────────────────────────────────

import { SessionBar } from "./SessionBar";
import { ChatPanel } from "./ChatPanel";
import { SessionHistory } from "./SessionHistory";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const CHANGE: ChangeEntry = {
  id: "c1",
  session_id: "s1",
  tool_name: "mac_flush_dns",
  description: "Flushed DNS cache",
  timestamp: Date.now(),
  undone: false,
};

const SESSION_WITH_CHANGES: SessionRecord = {
  id: "s1",
  created_at: new Date().toISOString(),
  ended_at: new Date().toISOString(),
  title: "Fixed DNS",
  message_count: 3,
  change_count: 2,
  resolved: true,
};

const mockSession = { startNewProblem: vi.fn() };

afterEach(() => cleanup());

beforeEach(() => {
  useSessionStore.setState({
    changes: [],
    changeLogOpen: false,
    historyOpen: false,
    pastSessions: [],
    sessionId: "s1",
    isActive: true,
    pendingApproval: null,
    knowledgeOpen: false,
    settingsOpen: false,
  });
  useChatStore.setState({ messages: [] });
  vi.clearAllMocks();
  vi.mocked(commands.listKnowledge).mockResolvedValue([]);
  vi.mocked(commands.listSessions).mockResolvedValue([]);
  vi.mocked(commands.getChanges).mockResolvedValue([]);
});

// ── SessionBar ───────────────────────────────────────────────────────────────

describe("SessionBar", () => {
  it("hides Actions button when there are no actions", () => {
    render(<SessionBar session={mockSession} />);
    expect(screen.queryByText(/Actions/)).toBeNull();
  });

  it("shows Actions button with correct count when actions exist", () => {
    useSessionStore.setState({ changes: [CHANGE] });
    render(<SessionBar session={mockSession} />);
    screen.getByText("Actions (1)");
  });

  it("shows plural count for multiple actions", () => {
    useSessionStore.setState({ changes: [CHANGE, { ...CHANGE, id: "c2" }] });
    render(<SessionBar session={mockSession} />);
    screen.getByText("Actions (2)");
  });

  it("opens ChangeLog when Actions button is clicked", async () => {
    useSessionStore.setState({ changes: [CHANGE] });
    render(<SessionBar session={mockSession} />);
    await userEvent.click(screen.getByText("Actions (1)"));
    expect(useSessionStore.getState().changeLogOpen).toBe(true);
  });

  it("applies active style when changeLogOpen is true", () => {
    useSessionStore.setState({ changes: [CHANGE], changeLogOpen: true });
    render(<SessionBar session={mockSession} />);
    const btn = screen.getByText("Actions (1)").closest("button")!;
    expect(btn.className).toContain("bg-accent");
  });
});

// ── ChangesBlock (tested through ChatPanel) ──────────────────────────────────

describe("ChangesBlock", () => {
  it("renders collapsed with action count when message has changeIds", async () => {
    useSessionStore.setState({ changes: [CHANGE] });
    useChatStore.setState({
      messages: [
        {
          id: "msg1",
          role: "assistant",
          content: "I fixed your DNS.",
          timestamp: Date.now(),
          changeIds: ["c1"],
        },
      ],
    });
    render(<ChatPanel />);
    await screen.findByText("1 action taken");
  });

  it("expands to show tool_name and description when clicked", async () => {
    useSessionStore.setState({ changes: [CHANGE] });
    useChatStore.setState({
      messages: [
        {
          id: "msg1",
          role: "assistant",
          content: "Done.",
          timestamp: Date.now(),
          changeIds: ["c1"],
        },
      ],
    });
    render(<ChatPanel />);
    await userEvent.click(await screen.findByText("1 action taken"));
    screen.getByText("mac_flush_dns");
    screen.getByText("Flushed DNS cache");
  });

  it("shows plural label for multiple actions", async () => {
    const change2: ChangeEntry = {
      ...CHANGE,
      id: "c2",
      tool_name: "mac_ping",
      description: "Pinged host",
    };
    useSessionStore.setState({ changes: [CHANGE, change2] });
    useChatStore.setState({
      messages: [
        {
          id: "msg1",
          role: "assistant",
          content: "Done.",
          timestamp: Date.now(),
          changeIds: ["c1", "c2"],
        },
      ],
    });
    render(<ChatPanel />);
    await screen.findByText("2 actions taken");
  });

  it("does not render when changeIds do not match any store changes", async () => {
    useSessionStore.setState({ changes: [] });
    useChatStore.setState({
      messages: [
        {
          id: "msg1",
          role: "assistant",
          content: "Nothing done.",
          timestamp: Date.now(),
          changeIds: ["c-ghost"],
        },
      ],
    });
    render(<ChatPanel />);
    await screen.findByText("Nothing done.");
    expect(screen.queryByText(/change made/)).toBeNull();
  });

  it("does not render when message has no changeIds", async () => {
    useChatStore.setState({
      messages: [
        {
          id: "msg1",
          role: "assistant",
          content: "Just checked your system.",
          timestamp: Date.now(),
        },
      ],
    });
    render(<ChatPanel />);
    await screen.findByText("Just checked your system.");
    expect(screen.queryByText(/change made/)).toBeNull();
  });
});

// ── SessionHistory actions badge ─────────────────────────────────────────────

describe("SessionHistory actions badge", () => {
  it("renders a clickable N actions badge for sessions with actions", async () => {
    vi.mocked(commands.listSessions).mockResolvedValue([SESSION_WITH_CHANGES]);
    useSessionStore.setState({ historyOpen: true });
    render(<SessionHistory />);
    await screen.findByText("2 actions");
  });

  it("calls getChanges with the session id when badge is clicked", async () => {
    vi.mocked(commands.listSessions).mockResolvedValue([SESSION_WITH_CHANGES]);
    vi.mocked(commands.getChanges).mockResolvedValue([CHANGE]);
    useSessionStore.setState({ historyOpen: true });
    render(<SessionHistory />);
    await userEvent.click(await screen.findByText("2 actions"));
    expect(commands.getChanges).toHaveBeenCalledWith("s1");
  });

  it("loads changes into store and opens ChangeLog when badge is clicked", async () => {
    vi.mocked(commands.listSessions).mockResolvedValue([SESSION_WITH_CHANGES]);
    vi.mocked(commands.getChanges).mockResolvedValue([CHANGE]);
    useSessionStore.setState({ historyOpen: true });
    render(<SessionHistory />);
    await userEvent.click(await screen.findByText("2 actions"));
    await waitFor(() => {
      expect(useSessionStore.getState().changeLogOpen).toBe(true);
      expect(useSessionStore.getState().changes).toEqual([CHANGE]);
    });
  });

  it("does not render a badge for sessions with zero actions", async () => {
    const noChanges: SessionRecord = { ...SESSION_WITH_CHANGES, change_count: 0 };
    vi.mocked(commands.listSessions).mockResolvedValue([noChanges]);
    useSessionStore.setState({ historyOpen: true });
    render(<SessionHistory />);
    await screen.findByText("Fixed DNS");
    expect(screen.queryByText(/action/)).toBeNull();
  });
});
