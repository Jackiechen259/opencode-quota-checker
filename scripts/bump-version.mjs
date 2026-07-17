#!/usr/bin/env node
/**
 * bump-version.mjs — single source of truth for releasing VOLC Status.
 *
 * Keeps package.json, src-tauri/tauri.conf.json, and src-tauri/Cargo.toml in
 * lockstep, then commits + tags the release. Pushing the tag triggers
 * .github/workflows/release.yml, which builds the Tauri app for all platforms
 * and publishes a GitHub Release with the artifacts.
 *
 * Usage:
 *   node scripts/bump-version.mjs <version|bump> [--push] [--dry-run]
 *
 * Examples:
 *   node scripts/bump-version.mjs 1.0.0          # explicit version
 *   node scripts/bump-version.mjs 1.0.0-rc1       # prerelease
 *   node scripts/bump-version.mjs patch           # 0.1.0 -> 0.1.1
 *   node scripts/bump-version.mjs minor           # 0.1.0 -> 0.2.0
 *   node scripts/bump-version.mjs major           # 0.1.0 -> 1.0.0
 *   node scripts/bump-version.mjs patch --push    # bump + commit + tag + push
 *
 * The git tag is always `v<version>` (e.g. v1.0.0). `--push` is required to
 * actually trigger the release workflow; without it the commit + tag stay local.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

// Project root: the script's parent dir by default, overridable via
// RELEASE_ROOT so it can be exercised against a throwaway checkout in tests.
const ROOT = process.env.RELEASE_ROOT
  ? resolve(process.env.RELEASE_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), "..");
const FILES = {
  "package.json": (src, v) => JSON.stringify({ ...src, version: v }, null, 2) + "\n",
  "src-tauri/tauri.conf.json": (src, v) =>
    JSON.stringify({ ...src, version: v }, null, 2) + "\n",
  "src-tauri/Cargo.toml": (src, v) =>
    src.replace(/^version\s*=\s*"[^"]*"/m, `version = "${v}"`),
};

function read(rel) {
  return readFileSync(resolve(ROOT, rel), "utf8");
}
function write(rel, content) {
  writeFileSync(resolve(ROOT, rel), content);
}
// execFileSync passes args directly to git without a shell, so messages with
// spaces / parens need no quoting. Works identically on Windows and POSIX.
function git(...args) {
  return execFileSync("git", args, { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function parseSemver(v) {
  const m = /^(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z0-9._-]+))?$/.exec(v);
  if (!m) return null;
  return { major: +m[1], minor: +m[2], patch: +m[3], pre: m[4] ?? null };
}
function bumpSemver(cur, kind) {
  const s = parseSemver(cur);
  if (!s) throw new Error(`Current version ${cur} is not semver`);
  switch (kind) {
    case "major":
      return `${s.major + 1}.0.0`;
    case "minor":
      return `${s.major}.${s.minor + 1}.0`;
    case "patch":
      return `${s.major}.${s.minor}.${s.patch + 1}`;
    default:
      throw new Error(`Unknown bump kind: ${kind}`);
  }
}

// --- args ---
const [, , ...rest] = process.argv;
const positional = rest.filter((a) => !a.startsWith("--"));
const flags = new Set(rest.filter((a) => a.startsWith("--")));
const dryRun = flags.has("--dry-run");
const push = flags.has("--push");

if (positional.length !== 1) {
  console.error("Usage: node scripts/bump-version.mjs <version|patch|minor|major> [--push] [--dry-run]");
  process.exit(2);
}
const arg = positional[0];

// --- resolve current + next version ---
const currentPkg = JSON.parse(read("package.json"));
const current = currentPkg.version;
const next = /^(patch|minor|major)$/.test(arg)
  ? bumpSemver(current, arg)
  : (parseSemver(arg) ? arg : null);

if (!next) {
  console.error(`Invalid version: ${arg} (expected semver like 1.2.3 or 1.2.3-rc1, or patch|minor|major)`);
  process.exit(2);
}
if (next === current) {
  console.error(`Version unchanged (${current}). Nothing to do.`);
  process.exit(1);
}

console.log(`Bumping ${current} -> ${next}${push ? " (and pushing)" : ""}`);

if (dryRun) {
  console.log("[dry-run] would update:", Object.keys(FILES).join(", "));
  console.log(`[dry-run] would commit + tag v${next}`);
  process.exit(0);
}

// --- update files ---
const updated = [];
for (const [rel, apply] of Object.entries(FILES)) {
  const raw = read(rel);
  let parsed;
  if (rel.endsWith(".json")) parsed = JSON.parse(raw);
  const out = rel.endsWith(".json") ? apply(parsed, next) : apply(raw, next);
  // sanity check: the new version actually appears
  if (!out.includes(`"${next}"`) && !out.includes(`version = "${next}"`)) {
    throw new Error(`Version ${next} not reflected in ${rel}; refusing to write`);
  }
  write(rel, out);
  updated.push(rel);
  console.log(`  updated ${rel}`);
}

// --- git: stage, commit, tag ---
const tag = `v${next}`;
// Make sure no stray working-tree changes ride along.
git("add", ...updated);
git("commit", "-m", `chore(release): ${tag}`);
// Annotated tag carries the version + a pointer back to this script.
git("tag", "-a", tag, "-m", `Release ${tag}`);

console.log(`\nCreated commit + tag ${tag}.`);

if (push) {
  const remote = git("remote").split("\n")[0] || "origin";
  git("push", remote, "HEAD");
  git("push", remote, tag);
  console.log(`Pushed to ${remote}. Release workflow will start shortly.`);
  console.log(`  https://github.com/${process.env.GITHUB_REPOSITORY || "Jackiechen259/volc_status"}/actions`);
} else {
  console.log("\nNot pushing. To trigger the release, run:");
  console.log(`  git push origin HEAD ${tag}`);
}
