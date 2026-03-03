#!/usr/bin/env node

import { existsSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";

const ROOT = process.cwd();
const TAURI_CONF_PATH = path.join(ROOT, "apps", "desktop", "src-tauri", "tauri.conf.json");
const BUNDLE_DIR = path.join(ROOT, "target", "release", "bundle");

function usage() {
  console.log(`Usage:
  node scripts/release.mjs --build [--tag vX.Y.Z] [--skip-install]
  node scripts/release.mjs --upload [--tag vX.Y.Z] [--skip-install]

Flags:
  --build         Build only (default when no mode is passed)
  --upload        Build and upload artifacts to GitHub release
  --tag           Override release tag (default: v{tauri.conf.json version})
  --skip-install  Skip 'pnpm install --frozen-lockfile'
  --help          Show this help
`);
}

function parseArgs(argv) {
  const result = {
    mode: "build",
    tag: "",
    skipInstall: false,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--build") {
      result.mode = "build";
      continue;
    }
    if (arg === "--upload") {
      result.mode = "upload";
      continue;
    }
    if (arg === "--skip-install") {
      result.skipInstall = true;
      continue;
    }
    if (arg === "--tag") {
      const next = argv[i + 1];
      if (!next) {
        throw new Error("Missing value for --tag");
      }
      result.tag = next;
      i += 1;
      continue;
    }
    if (arg === "--help" || arg === "-h") {
      usage();
      process.exit(0);
    }
    throw new Error(`Unknown argument: ${arg}`);
  }

  return result;
}

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      stdio: "inherit",
      shell: process.platform === "win32",
      cwd: ROOT,
    });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}`));
      }
    });
  });
}

function hasCommand(command) {
  const checker = process.platform === "win32" ? "where" : "which";
  return new Promise((resolve) => {
    const child = spawn(checker, [command], {
      stdio: "ignore",
      shell: process.platform === "win32",
    });
    child.on("close", (code) => resolve(code === 0));
    child.on("error", () => resolve(false));
  });
}

async function readVersion() {
  if (!existsSync(TAURI_CONF_PATH)) {
    throw new Error(`Missing ${TAURI_CONF_PATH}`);
  }
  const raw = await readFile(TAURI_CONF_PATH, "utf8");
  const json = JSON.parse(raw);
  if (!json.version) {
    throw new Error(`Could not read version from ${TAURI_CONF_PATH}`);
  }
  return String(json.version);
}

async function collectArtifacts() {
  const candidates = [
    ["dmg", ".dmg"],
    ["macos", ".tar.gz"],
    ["msi", ".msi"],
    ["nsis", ".exe"],
    ["deb", ".deb"],
    ["appimage", ".AppImage"],
  ];

  const artifacts = [];
  for (const [subdir, suffix] of candidates) {
    const dir = path.join(BUNDLE_DIR, subdir);
    if (!existsSync(dir)) continue;
    const files = await readdir(dir, { withFileTypes: true });
    for (const f of files) {
      if (!f.isFile()) continue;
      if (f.name.endsWith(suffix)) {
        artifacts.push(path.join(dir, f.name));
      }
    }
  }
  artifacts.sort();
  return artifacts;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const version = await readVersion();
  const tag = args.tag || `v${version}`;
  const uploading = args.mode === "upload";

  console.log(`==> Building itman ${tag} on ${process.platform}/${process.arch}`);

  if (!(await hasCommand("pnpm"))) {
    throw new Error("Missing required command: pnpm");
  }

  if (!args.skipInstall) {
    console.log("==> Installing dependencies...");
    await runCommand("pnpm", ["install", "--frozen-lockfile"]);
  } else {
    console.log("==> Skipping dependency install (--skip-install)");
  }

  console.log("==> Running tauri build...");
  await runCommand("pnpm", ["--filter", "@itman/desktop", "tauri", "build"]);

  const artifacts = await collectArtifacts();
  if (artifacts.length === 0) {
    throw new Error(`No build artifacts found in ${BUNDLE_DIR}`);
  }

  console.log("==> Artifacts:");
  for (const artifact of artifacts) {
    console.log(`    ${artifact}`);
  }

  if (!uploading) {
    console.log("==> Build-only mode complete.");
    return;
  }

  if (!(await hasCommand("gh"))) {
    throw new Error("Missing required command: gh");
  }

  console.log(`==> Uploading to GitHub release ${tag}...`);
  let releaseExists = true;
  try {
    await runCommand("gh", ["release", "view", tag]);
  } catch {
    releaseExists = false;
  }

  if (!releaseExists) {
    await runCommand("gh", ["release", "create", tag, "--title", `itman ${tag}`, "--generate-notes"]);
  }

  await runCommand("gh", ["release", "upload", tag, ...artifacts, "--clobber"]);
  await runCommand("gh", ["release", "view", tag, "--json", "url", "-q", ".url"]);
}

main().catch((error) => {
  console.error(`ERROR: ${error.message}`);
  process.exit(1);
});
