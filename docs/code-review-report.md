# Code Quality Review Report

**Date:** 2026-03-10
**Reviewer:** Claude Opus 4.6 + OpenAI Codex gpt-5.3
**Scope:** Full codebase — Rust backend + TypeScript frontend

## Summary

Two rounds of automated review were performed. 18 findings were identified across security, error handling, performance, consistency, and dead code categories. **10 were fixed** in two commits; 8 are deferred (architectural security changes that need design discussion).

## Fixes Applied

### Commit 1: `58fe6b4` — Code quality improvements from automated review

| Fix | Category | Impact |
|-----|----------|--------|
| Handle mutex poisoning in scanner commands | Error handling | Prevents app crash if a scanner thread panics |
| `Vec::remove(0)` → `VecDeque::pop_front()` in disk scanner | Performance | O(1) vs O(n) queue dequeue |
| Add `catch`/`finally` to API key check | Error handling | Splash screen always dismisses, even on IPC error |
| Gate elapsed timer on active status | Performance | Stops 1Hz re-renders when app is idle |
| Add 12 missing Linux tool i18n labels | Consistency | Linux users see proper tool names instead of "Working..." |
| Add `activate_playbook` + `web_fetch` to tool name set | Consistency | New tools show localized status |

### Commit 2: `d75e51d` — Correct tool name mismatches and harden scanner

| Fix | Category | Impact |
|-----|----------|--------|
| Rename `search_knowledge`→`knowledge_search`, `read_knowledge`→`knowledge_read` | **Bug fix** | Frontend now matches actual backend tool names |
| Remove nonexistent `list_knowledge` from i18n and test allowlist | Dead code | Removes references to a tool that doesn't exist |
| Handle poisoned locks in scanner manager | Error handling | Background scanner skips instead of crashing |
| Record failed job on triggered scan errors | Error handling | On-demand scan failures now visible in UI |
| `while let` queue loop | Safety | Eliminates `unwrap()`, pushes dir back on budget break |
| Remove dead `elapsedRef` | Dead code | Write-only ref was never read |

## Deferred Items (Need Design Discussion)

### Security (High Priority, Architectural)

1. **PowerShell command injection** — User-controlled strings interpolated into PS scripts. Fix requires centralized escaping helper or arg-based invocation. Affects: `windows/network.rs`, `windows/apps.rs`, `windows/diagnostics.rs`.

2. **`shell_run` blacklist-based approval** — Substring-based dangerous command checks are bypassable. Fix requires moving to allowlist-first auto-approval model. Affects all platform `diagnostics.rs` files.

3. **Over-broad readable path prefixes** — Diagnostics tools allow reading from broad directory prefixes that could expose sensitive files (`.ssh`, tokens, certs). Needs explicit denylist.

4. **Feedback trace exfiltration** — Recent LLM traces included in bug report payloads may contain sensitive values. Needs redaction before composing feedback.

5. **`write_secret` accepts arbitrary paths** — Should restrict to app-owned config directories.

### Performance (Medium, Can Optimize Later)

6. **Scanner holds DB lock during expensive work** — Should do I/O outside lock, persist in short critical section.

7. **Journal delete+insert without transaction** — Batch writes would reduce SQLite overhead.

### Consistency (Low)

8. **Frontend/backend timestamp type mismatch** — Frontend expects `number`, backend returns `String`. Align to ISO string.

## Recommendations

- **Security items 1-2** should be addressed before any public release. The PowerShell injection and shell_run blacklist are the highest-risk surfaces.
- **Items 3-5** are important but lower risk since they require LLM cooperation (the LLM would have to intentionally exploit them).
- **Performance items 6-7** are optimization opportunities, not correctness issues.
- **ChangeLog component** is not dead code — it has store wiring and should be re-integrated into the UI when the actions/changes feature is prioritized.
- **Verifier module** is a clearly marked placeholder — keep it as a reminder for the planned verification system.
