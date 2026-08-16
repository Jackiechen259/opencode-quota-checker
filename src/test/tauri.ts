// Test doubles for the Tauri API surface, shared by frontend tests.
//
// Importing this module registers the `vi.mock` factories (they live in the
// same module as the doubles they reference) and exports the doubles so
// tests can route `invoke` by command name and emit events:
//
//   import { tauri } from "../test/tauri"; // must precede app imports
//   tauri.invoke.mockImplementation(async (command) => { ... });
//   tauri.emit("app://status", payload);

import { vi } from "vitest";

const invoke = vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(
  async () => {
    throw new Error("no invoke mock implementation registered");
  },
);

const windowApi = {
  label: "main",
  minimize: vi.fn(async () => {}),
  toggleMaximize: vi.fn(async () => {}),
  close: vi.fn(async () => {}),
  hide: vi.fn(async () => {}),
  isMaximized: vi.fn(async () => false),
  onResized: vi.fn(async () => () => {}),
  onMoved: vi.fn(async () => () => {}),
};

const eventListeners = new Map<string, Array<(message: { payload: unknown }) => void>>();
const listen = vi.fn(
  async (event: string, handler: (message: { payload: unknown }) => void) => {
    const handlers = eventListeners.get(event) ?? [];
    handlers.push(handler);
    eventListeners.set(event, handlers);
    return () => {
      const list = eventListeners.get(event) ?? [];
      const index = list.indexOf(handler);
      if (index >= 0) list.splice(index, 1);
    };
  },
);

function emit(event: string, payload: unknown) {
  for (const handler of eventListeners.get(event) ?? []) handler({ payload });
}

function reset() {
  invoke.mockReset();
  windowApi.minimize.mockClear();
  windowApi.toggleMaximize.mockClear();
  windowApi.close.mockClear();
  windowApi.hide.mockClear();
  windowApi.isMaximized.mockClear();
}

export const tauri = { invoke, windowApi, listen, emit, reset };

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => windowApi }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: vi.fn(async () => {}),
}));
