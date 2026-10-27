// Regression guard for the caption-button stacking contract.
//
// Bug it protects against: `.titlebar` used to carry `position: relative;
// z-index: 1`. That `z-index` created a stacking context, which trapped
// `.titlebar-controls` (z-index 60) *inside* level 1 of the root stacking
// context — below the fixed resize handles (z-index 50). The top-right 12×12
// "resize north-east" handle then sat on top of the close button, so aiming at
// the window corner (how every Windows user closes a window) started a resize
// instead of a close. Measured: 83 of 368 sample points across the 46×32 close
// button were intercepted by a resize handle.
//
// jsdom has no layout engine, so the geometry cannot be asserted here. What is
// asserted is the invariant that makes the geometry correct: the caption
// controls outrank the resize handles in the *same* stacking context, and
// nothing between the window root and the controls creates a stacking context
// of its own.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// Paths are resolved against the project root (vitest's cwd). Importing the
// CSS through Vite is not an option: vitest replaces every `.css` module —
// `?raw` included — with an empty string unless `test.css` is enabled.
/** Comments are stripped first: they are prose and may contain colons or
 * braces, either of which would break the naive declaration scan below. */
function readStyleSheet(name: string): string {
  return readFileSync(`src/styles/${name}`, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

/** Returns the declaration block of the first rule with exactly `selector`. */
function ruleBody(css: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  // `^` + optional whitespace pins the selector to a rule start, so
  // ".titlebar" never matches ".titlebar-controls" or ".titlebar-brand".
  const match = new RegExp(`(?:^|\\n)\\s*${escaped}\\s*\\{([^}]*)\\}`, "m").exec(css);
  if (!match) throw new Error(`rule not found in stylesheet: ${selector}`);
  return match[1];
}

function declarations(body: string): Map<string, string> {
  const map = new Map<string, string>();
  for (const raw of body.split(";")) {
    const index = raw.indexOf(":");
    if (index < 0) continue;
    const property = raw.slice(0, index).trim();
    if (!property) continue;
    map.set(property, raw.slice(index + 1).trim());
  }
  return map;
}

function zIndex(css: string, selector: string): number {
  const value = declarations(ruleBody(css, selector)).get("z-index");
  if (value === undefined) throw new Error(`${selector} has no z-index`);
  const parsed = Number.parseInt(value, 10);
  if (Number.isNaN(parsed)) throw new Error(`${selector} z-index is not numeric: ${value}`);
  return parsed;
}

const mainWindow = readStyleSheet("main-window.css");
const tokens = readStyleSheet("tokens.css");

describe("titlebar stacking contract", () => {
  it("caption controls outrank the resize handles", () => {
    expect(zIndex(mainWindow, ".titlebar-controls")).toBeGreaterThan(
      zIndex(mainWindow, ".resize-handle"),
    );
  });

  it("no stacking context is created between the window root and the controls", () => {
    // Any z-index (or a transform/filter/contain) on an ancestor re-scopes the
    // controls, silently dropping them back below the handles.
    for (const selector of [".main-window", ".main-frame", ".titlebar"]) {
      const body = ruleBody(mainWindow, selector);
      expect(declarations(body).has("z-index"), `${selector} must not set z-index`).toBe(false);
      expect(body, `${selector} must not set transform`).not.toMatch(/\btransform\s*:/);
      expect(body, `${selector} must not set filter`).not.toMatch(/\bfilter\s*:/);
    }
  });

  it("the close-button corner is not claimed by a resize handle", () => {
    // The north-east corner is the close button; a diagonal-resize handle there
    // can only steal the click.
    expect(declarations(ruleBody(mainWindow, ".resize-handle-ne")).get("display")).toBe("none");
  });

  it("the top resize strip stops where the caption buttons begin", () => {
    expect(declarations(ruleBody(mainWindow, ".resize-handle-n")).get("right")).toBe(
      "var(--titlebar-controls-width)",
    );
  });

  it("the caption-button width token spans all three controls", () => {
    const body = ruleBody(tokens, ":root");
    const map = declarations(body);
    expect(map.get("--titlebar-control-width")).toBe("46px");
    expect(map.get("--titlebar-controls-width")).toBe(
      "calc(var(--titlebar-control-width) * 3)",
    );
    // The controls must consume the same token, or the strip and the buttons
    // drift apart.
    expect(declarations(ruleBody(mainWindow, ".titlebar-control")).get("width")).toBe(
      "var(--titlebar-control-width)",
    );
  });
});
