# Playbooks Feature — Manual Testing Guide

## Prerequisites

1. You're on the `feature/playbooks` branch
2. Dev app builds and runs: `cd apps/desktop && npm run tauri dev`
3. API key or proxy is configured (Noah can call Claude)

---

## Test 1: Bootstrap — Playbooks directory created on first run

**Steps:**
1. Delete the playbooks directory if it exists:
   ```bash
   rm -rf ~/Library/Application\ Support/com.noah.app/playbooks/
   ```
2. Launch the dev app: `cd apps/desktop && npm run tauri dev`
3. Check the directory was created:
   ```bash
   ls -la ~/Library/Application\ Support/com.noah.app/playbooks/
   ```

**Expected:**
- Directory exists with 5 `.md` files:
  - `app-doctor.md`
  - `disk-space-recovery.md`
  - `network-diagnostics.md`
  - `performance-forensics.md`
  - `printer-repair.md`
- Each file has YAML frontmatter with `name:` and `description:` fields

**Pass criteria:** All 5 files present, each starts with `---\nname: ...\ndescription: ...\n---`

---

## Test 2: Playbook activation — Network diagnostics

**Steps:**
1. Open the app
2. Open the Debug Panel (Cmd+D)
3. Type: **"my Wi-Fi keeps dropping every few minutes"**
4. Watch the debug panel for tool calls

**Expected:**
- In the debug panel, you should see an `activate_playbook` tool call with input `{"name": "network-diagnostics"}`
- The tool result should contain the full network diagnostics protocol (mentions "Step 1: Quick connectivity check", etc.)
- Noah should then follow the protocol, starting with `mac_ping` to `8.8.8.8`
- Subsequent tool calls should follow the playbook's step-by-step order

**Pass criteria:** `activate_playbook` called, full protocol returned, Noah follows it systematically

---

## Test 3: Playbook activation — Performance

**Steps:**
1. Start a new session
2. Type: **"my Mac is really slow and the fans are going crazy"**
3. Watch the debug panel

**Expected:**
- `activate_playbook` called with `{"name": "performance-forensics"}`
- Noah runs `mac_system_info` and `mac_process_list` as the protocol's Step 1 dictates
- Noah classifies the situation (CPU-bound, memory pressure, etc.) based on results

**Pass criteria:** Correct playbook activated, diagnostic tools called in protocol order

---

## Test 4: Playbook activation — Disk space

**Steps:**
1. Start a new session
2. Type: **"I keep getting 'disk full' warnings, I can't even install updates"**
3. Watch the debug panel

**Expected:**
- `activate_playbook` called with `{"name": "disk-space-recovery"}`
- Noah runs `mac_disk_usage` and `disk_audit` (new compound tool)
- Results show categorized breakdown of space usage

**Pass criteria:** Correct playbook activated, `disk_audit` tool produces categorized output

---

## Test 5: Playbook activation — Printer

**Steps:**
1. Start a new session
2. Type: **"my printer isn't working, print jobs are stuck"**
3. Watch the debug panel

**Expected:**
- `activate_playbook` called with `{"name": "printer-repair"}`
- Noah runs `mac_print_queue` first (Step 1 of protocol)

**Pass criteria:** Correct playbook activated, follows printer protocol

---

## Test 6: Playbook activation — App crashes

**Steps:**
1. Start a new session
2. Type: **"Safari keeps crashing every time I open it"**
3. Watch the debug panel

**Expected:**
- `activate_playbook` called with `{"name": "app-doctor"}`
- Noah runs `mac_app_list` and/or `crash_log_reader` with `{"app_name": "Safari"}`

**Pass criteria:** Correct playbook activated, crash log reader attempted

---

## Test 7: No playbook for simple questions (negative test)

**Steps:**
1. Start a new session
2. Type: **"what's my IP address?"**
3. Watch the debug panel

**Expected:**
- Noah should call `mac_network_info` directly — **no** `activate_playbook` call
- Simple question gets a direct answer

**Pass criteria:** No `activate_playbook` in the debug log

---

## Test 8: No playbook for greetings (negative test)

**Steps:**
1. Start a new session
2. Type: **"hi there!"**
3. Watch the debug panel

**Expected:**
- Noah responds with a greeting — no tool calls at all
- No `activate_playbook`

**Pass criteria:** No playbook activation for casual conversation

---

## Test 9: Compound tool — wifi_scan

**Steps:**
1. Start a new session
2. Type: **"scan my Wi-Fi environment and check for interference"**
3. Watch the debug panel

**Expected:**
- `wifi_scan` tool called (may be directly or via a playbook)
- Output includes: SSID, signal (RSSI) in dBm, noise, channel, signal quality assessment, nearby networks list

**Pass criteria:** `wifi_scan` returns structured Wi-Fi data

---

## Test 10: Compound tool — disk_audit

**Steps:**
1. Start a new session
2. Type: **"what's eating up my disk space?"**
3. Watch the debug panel

**Expected:**
- `disk_audit` called (likely via disk-space-recovery playbook)
- Output lists directories sorted by size with human-readable sizes (e.g., "15.2 GB  Xcode DerivedData")
- Time Machine snapshot count shown if any exist

**Pass criteria:** Categorized space breakdown returned

---

## Test 11: Compound tool — crash_log_reader

**Steps:**
1. Start a new session
2. Type: **"check if there are any crash reports for Safari"**
3. Watch the debug panel

**Expected:**
- `crash_log_reader` called with `{"app_name": "Safari"}`
- If crash reports exist: summary with exception type, crashed thread, top stack frames
- If no crash reports: message saying "No crash reports found for 'safari'"

**Pass criteria:** Tool runs without error, returns appropriate result

---

## Test 12: crash_log_reader with log_path

**Steps:**
1. Start a new session
2. Type: **"show me the recent CUPS error log"**
3. Watch the debug panel

**Expected:**
- `crash_log_reader` called with `{"log_path": "/var/log/cups/error_log"}`
- Returns the last 100 lines of the CUPS log (or an error if the file doesn't exist/is empty)

**Pass criteria:** Tool reads the specified log file

---

## Test 13: Custom playbook — pluggability

**Steps:**
1. Create a custom playbook:
   ```bash
   cat > ~/Library/Application\ Support/com.noah.app/playbooks/test-playbook.md << 'PLAYBOOK'
   ---
   name: test-custom
   description: A test playbook to verify custom playbook loading
   ---

   # Test Custom Playbook

   ## When to activate
   User says "run the test playbook" or "test custom playbook".

   ## Protocol

   ### Step 1: Say hello
   Respond with: "Custom playbook loaded successfully!"

   ### Step 2: Check system
   Run `mac_system_info` to get basic system details.
   PLAYBOOK
   ```
2. **Restart the app** (quit and relaunch — playbooks scan on startup)
3. Type: **"run the test playbook"**
4. Watch the debug panel

**Expected:**
- `activate_playbook` called with `{"name": "test-custom"}`
- Full custom playbook content returned
- Noah follows the custom protocol

**Pass criteria:** Custom playbook detected, activatable, and followed

---

## Test 14: Custom playbook appears in context

**Steps:**
1. With the custom playbook from Test 13 still in place, restart the app
2. In the debug panel, look at the system prompt (first `llm_request` event detail)
3. Or type: **"what playbooks do you have available?"**

**Expected:**
- The system prompt should contain a "Playbooks" section listing 6 playbooks (5 built-in + 1 custom)
- `test-custom` should appear in the list with its description

**Pass criteria:** Custom playbook visible in the available playbooks list

---

## Test 15: Built-in playbooks not overwritten on restart

**Steps:**
1. Edit a built-in playbook:
   ```bash
   echo "CUSTOM EDIT" >> ~/Library/Application\ Support/com.noah.app/playbooks/network-diagnostics.md
   ```
2. Restart the app
3. Check the file:
   ```bash
   tail -1 ~/Library/Application\ Support/com.noah.app/playbooks/network-diagnostics.md
   ```

**Expected:**
- The last line should still be "CUSTOM EDIT" — the bootstrap did NOT overwrite the existing file

**Pass criteria:** User edits preserved across restarts

---

## Test 16: activate_playbook error handling

**Steps:**
1. Start a new session
2. In a scenario where Noah might try a wrong name, or directly test:
   - If you can craft a message that makes Noah call `activate_playbook` with a nonexistent name

**Alternative — unit test verification:**
```bash
cd apps/desktop/src-tauri && cargo test playbooks -- --nocapture
```

**Expected:**
- The `test_read_playbook_not_found` test passes
- Error message includes "not found" and lists available playbook names

**Pass criteria:** Graceful error with helpful available-names list

---

## Cleanup

After testing, remove the custom test playbook:
```bash
rm ~/Library/Application\ Support/com.noah.app/playbooks/test-playbook.md
```

---

## Quick Smoke Test (if short on time)

Run these 3 tests for minimum coverage:
1. **Test 1** (bootstrap) — verifies infrastructure works
2. **Test 2** (network diagnostics) — verifies end-to-end playbook activation
3. **Test 7** (simple question) — verifies no regression for non-playbook queries
