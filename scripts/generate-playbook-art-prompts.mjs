#!/usr/bin/env node

import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const PLAYBOOK_DIR = path.join(ROOT, "apps", "desktop", "src-tauri", "playbooks");
const OUTPUT = path.join(ROOT, "scripts", "playbook-art-prompts.json");

function parseFrontmatter(md) {
  const trimmed = md.trimStart();
  if (!trimmed.startsWith("---")) {
    return {};
  }

  const rest = trimmed.slice(3);
  const end = rest.indexOf("\n---");
  if (end === -1) {
    return {};
  }

  const yaml = rest.slice(0, end);
  const meta = {};
  for (const line of yaml.split("\n")) {
    const [rawKey, ...rawValue] = line.split(":");
    if (!rawKey || rawValue.length === 0) continue;
    const key = rawKey.trim();
    const value = rawValue.join(":").trim();
    meta[key] = value;
  }
  return meta;
}

function buildPrompt({ name, description, platform }) {
  return [
    `Create a clean, modern illustration card for a troubleshooting playbook titled "${name}".`,
    `Concept: ${description || "IT diagnostic workflow"}.`,
    `Platform context: ${platform || "all"}.`,
    "Style: flat vector, soft gradients, rounded corners, no text in image, subtle depth, product UI aesthetic.",
    "Composition: one clear hero object + 1-2 supporting objects representing diagnosis and repair.",
    "Palette: calm tech tones (blue/slate/teal) with a single accent color.",
    "Export target: 16:9 thumbnail card for desktop app knowledge gallery.",
  ].join(" ");
}

async function main() {
  const files = (await readdir(PLAYBOOK_DIR)).filter((file) => file.endsWith(".md"));
  const prompts = [];

  for (const file of files) {
    const full = path.join(PLAYBOOK_DIR, file);
    const md = await readFile(full, "utf8");
    const meta = parseFrontmatter(md);

    const normalized = {
      name: meta.name || file.replace(/\.md$/, "").replace(/-/g, " "),
      description: meta.description || "",
      platform: meta.platform || "all",
    };

    prompts.push({
      file,
      ...normalized,
      prompt: buildPrompt(normalized),
    });
  }

  await writeFile(OUTPUT, `${JSON.stringify(prompts, null, 2)}\n`, "utf8");
  console.log(`Wrote ${prompts.length} prompt(s) to ${path.relative(ROOT, OUTPUT)}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
