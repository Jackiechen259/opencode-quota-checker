// Minimal Node typing for the stylesheet-inspection test.
//
// `@types/node` is intentionally NOT a dependency of this project (the app is
// browser code; only the Rust side does I/O), and adding it just so one test
// can read a CSS file would be a heavyweight change. Vitest runs on Node, so
// the module exists at runtime — only the type is missing. This declares the
// single function that is actually used, with its real signature.
//
// If the project ever grows real Node tooling, add `@types/node` and delete
// this file: a hand-written declaration must never shadow the real one.

declare module "node:fs" {
  export function readFileSync(path: string, encoding: "utf8"): string;
}
