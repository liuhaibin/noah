---
name: setup-nanoclaw
description: Install and configure NanoClaw — a WhatsApp AI assistant powered by Claude
platform: macos
last_reviewed: 2026-03-08
author: noah-team
type: system
---

# Set Up NanoClaw

NanoClaw is a WhatsApp AI assistant powered by Claude. This playbook guides
the user through installing, authenticating, and starting the service.

## When to activate
User wants to install NanoClaw specifically (mentions "nanoclaw" by name).
If the user says "openclaw", use `setup-openclaw` instead — OpenClaw is
the successor product with broader channel support and a built-in gateway.

## Step 1: Check Environment

Run `mac_run_command` with `node --version` to check Node.js.
- Need Node.js 22+. If missing or old, activate `setup-nanoclaw/install-node`.
- Check `which container` (Apple Container) and `docker info` (Docker) availability.
- Check if `~/nanoclaw` or `~/Playground/nanoclaw` exists already.

## Step 2: Clone and Install

If NanoClaw isn't cloned yet:
```
git clone https://github.com/qwibitai/nanoclaw.git ~/nanoclaw
cd ~/nanoclaw && npm install
```
If already cloned, run `cd ~/nanoclaw && git pull && npm install`.

## Step 3: Container Runtime

Check which container runtime is available:
- **Apple Container** (`container --version`): preferred on macOS Sequoia+
- **Docker** (`docker info`): fallback

If neither is available, guide the user to install Docker Desktop or
update to macOS 26+ for Apple Container.

Build the agent container:
```
cd ~/nanoclaw && bash container/build.sh
```

## Step 4: Claude Authentication

Ask how the user wants to authenticate Claude:

**Option A: Claude subscription (OAuth)** — recommended for personal use.
Run `cd ~/nanoclaw && npx @anthropic-ai/claude-code --dangerously-skip-permissions`
and tell user to complete the browser login. Use WAIT_FOR_USER.

**Option B: Anthropic API key** — for API users.
Collect the API key via `secure_input` (secret_name: "anthropic_api_key").
Write to `~/nanoclaw/.env` using `write_secret` with format:
```
ANTHROPIC_API_KEY={{value}}
```

## Step 5: WhatsApp Authentication

This is the trickiest step. Tell the user:

> I need to connect NanoClaw to your WhatsApp. You'll scan a QR code
> in your phone's WhatsApp settings (Linked Devices > Link a Device).

Run: `cd ~/nanoclaw && node -e "require('./src/auth').authenticate()"` or
the appropriate auth script. Use WAIT_FOR_USER — the user needs to scan
the QR code on their phone.

After auth succeeds, verify: `ls ~/nanoclaw/store/auth/` should contain
credential files.

## Step 6: Configure Trigger Word

Ask the user what trigger word they want. Default: "Andy".
Use `text_input` (placeholder: "e.g., Andy, Hey Bot, Assistant").

Write to `.env`:
```
TRIGGER_WORD=<user's choice>
```

## Step 7: Start the Service

Register as a launchd service for auto-start:
```
cd ~/nanoclaw && node scripts/register-service.js
```

Or start manually for testing first:
```
cd ~/nanoclaw && npm start
```

Verify it's running by checking the logs.

## Step 8: Verify

Send a test message on WhatsApp using the trigger word.
Check `~/nanoclaw/logs/nanoclaw.log` for activity.

If everything works, show a done card summarizing:
- NanoClaw location
- Trigger word
- Container runtime
- Auth method
- How to check logs / restart

## Available Modules

After the core setup, the user can add optional features. Each is a
separate playbook that can be activated independently:

- **setup-nanoclaw/add-telegram** — Add Telegram as a messaging channel
  (alongside or instead of WhatsApp)
- **setup-nanoclaw/add-gmail** — Add Gmail as a tool or full email channel
- **setup-nanoclaw/add-voice** — Transcribe WhatsApp voice messages using
  OpenAI Whisper
- **setup-nanoclaw/add-parallel** — Add web research via Parallel AI
- **setup-nanoclaw/add-x** — Post to X/Twitter via browser automation

When setup is complete, ask the user if they'd like to configure any
of these optional modules.

## Escalation
If container build fails repeatedly, check Docker/Apple Container logs.
If WhatsApp auth fails, the user may need to update WhatsApp or try
the pairing code method instead of QR.
