# Next-Gen UI Plan for Noah

**Date:** 2026-03-10
**Context:** Inspired by Claude Cowork Desktop and OpenAI Codex Desktop, adapted for IT support agent use case.

## Design Principle (Unchanged)

> Any UI is a transparent "representation" of user interaction as if they are directly talking to the underlying model.

All UI elements below preserve this principle. No UI should create semantic mismatch between what the user sees and what the LLM thread contains.

---

## Phase 1: Live Step Checklist (High Impact, Low Effort)

**Inspiration:** Claude Cowork's real-time step checklist with strikethrough

Noah already parses `## Step N: Label` headers from playbooks for progress. Currently this renders as a simple `Step 2 of 6` indicator. Upgrade to:

- A **persistent sidebar checklist** showing all detected steps
- Steps strike through with a checkmark as they complete
- Current step is highlighted with a spinner
- Failed steps show error state
- Clicking a completed step scrolls to that point in chat

**Why it fits:** Direct extension of existing `parse_steps()` infrastructure. Zero LLM changes. The checklist is a transparent view of the conversation's progress.

**Implementation:**
- New `PlaybookProgress` component (or extend `SessionSummary`)
- Consume existing `debug-log` events (`step_progress` type)
- Render as a collapsible panel or inline in the existing sidebar
- ~200 lines of React, no Rust changes

---

## Phase 2: Plan Review Gate (High Impact, Medium Effort)

**Inspiration:** Cowork's "Let it run" approval pattern

When Noah activates a playbook, instead of immediately starting execution:

1. Show the full step plan as a review card
2. User clicks **"Let it run"** once
3. Noah executes all `RUN_STEP` actions without per-step approval
4. Only `WAIT_FOR_USER` steps still pause (by design — these require human action)
5. Destructive actions (Tier 3) still require individual approval

**Why it fits:** Reduces approval fatigue. Currently users click "Continue" for every step. The plan review replaces N approvals with 1. Transparent because the plan IS the conversation context.

**Implementation:**
- New `PlanReviewCard` component
- Orchestrator flag: `auto_approve_run_steps` set after plan approval
- Approval logic in `execute_tool` already has `NeedsApproval` vs `ReadOnly` tiers
- ~150 lines React + ~30 lines Rust

---

## Phase 3: Rich Diagnostic Panel (Medium Impact, Medium Effort)

**Inspiration:** Claude's Artifacts side panel + Codex's diff review

Instead of dumping diagnostic output as text in chat, render structured results in a dedicated side panel:

- **System info card:** CPU, RAM, disk, OS version in a clean dashboard
- **Network status card:** ping results, DNS, connectivity as traffic-light indicators
- **Before/after diffs:** When Noah modifies a config file, show the diff in a side panel
- **Process table:** Interactive sortable table instead of text dump

**Why it fits:** The panel displays the same data that would be in chat, just formatted. The conversation still has the raw data. This is presentation-layer only.

**Implementation:**
- New `DiagnosticArtifact` component that renders structured tool outputs
- Tool output already includes JSON in `detail` field — parse and render
- Side panel layout (right pane that opens on structured output)
- ~400 lines React, no Rust changes (tool outputs already have the data)

---

## Phase 4: Inline Terminal View (Medium Impact, Low Effort)

**Inspiration:** Codex's per-thread terminal

Advanced users want to see what commands Noah is actually running. Add a collapsible terminal view below the chat:

- Shows `shell_run` commands and their stdout/stderr in real time
- Read-only (Noah controls execution)
- Monospace font, ANSI color support
- Collapsible — hidden by default, toggle with Cmd+T or button

**Why it fits:** Pure transparency layer. Shows exactly what the LLM is doing. Already have all the data via `debug-log` events.

**Implementation:**
- Extend existing `DebugPanel` or create a simpler `TerminalView`
- Filter `debug-log` events for `tool_call` + `tool_result` where tool is `shell_run`
- ~150 lines React

---

## Phase 5: Scheduled Tasks UI (Medium Impact, Higher Effort)

**Inspiration:** Codex's Automations

Noah already has proactive monitoring (6h background scan cycle). Give users control:

- **Schedule builder:** "Check disk space every day at 9am"
- **Task inbox:** Proactive findings appear as cards, not just banners
- **History view:** Past automation runs with results
- **Custom checks:** User defines what to monitor ("alert me if Docker is down")

**Why it fits:** Extends existing `proactive/mod.rs` infrastructure. The schedule is user-defined, the execution is the same LLM conversation pattern.

**Implementation:**
- New `AutomationsPanel` view (fourth sidebar tab)
- Backend: `cron`-style scheduler in `proactive/mod.rs`
- DB: `automations` table (schedule, prompt, last_run, results)
- ~500 lines React + ~200 lines Rust

---

## Phase 6: Pop-Out Floating Windows (Low Impact, Medium Effort)

**Inspiration:** Codex's detachable thread windows

Let users pop out a running playbook into a floating always-on-top mini window while doing other work:

- Mini view shows: current step, progress bar, status
- Click to expand back to full window
- Useful for long-running playbooks (CUDA install, Windows updates)

**Why it fits:** No semantic change. Just a different viewport for the same conversation.

**Implementation:**
- Tauri `WebviewWindow::new()` for pop-out
- Shared state via Tauri events (already used for `debug-log`)
- ~200 lines React + ~50 lines Rust

---

## Priority Matrix

| Phase | Impact | Effort | Priority |
|-------|--------|--------|----------|
| 1. Live Step Checklist | High | Low | **Do first** |
| 2. Plan Review Gate | High | Medium | **Do second** |
| 3. Rich Diagnostic Panel | Medium | Medium | Do third |
| 4. Inline Terminal | Medium | Low | Quick win, do anytime |
| 5. Scheduled Tasks | Medium | High | Plan for v0.16+ |
| 6. Pop-Out Windows | Low | Medium | Nice-to-have |

## What NOT to Build

- **Multi-project/multi-thread** (Codex) — Noah is single-purpose IT support, not a dev IDE. One conversation at a time is the right model.
- **Folder-scoped sandbox** (Cowork) — Noah operates on system config, not user files. Scoping to a folder doesn't match the use case.
- **MCP Apps / iframe widgets** — Too much infrastructure for current scale. Noah's tool system is simpler and more controlled.
- **IDE context sync** — Noah isn't a code tool. System context (OS, installed apps) is auto-detected, not user-selected.
- **Git integration** (Codex) — Not relevant for IT support workflows.

---

## Architecture Notes

All phases preserve the core architecture:
- LLM emits tool calls → Orchestrator executes → Frontend renders
- UI is a transparent view of the conversation thread
- No "wrapper" UI that creates semantic mismatch
- All new panels consume existing data (tool outputs, debug-log events)
- No new backend APIs needed for Phases 1, 3, 4 (pure frontend)
