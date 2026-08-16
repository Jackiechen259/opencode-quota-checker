// Fixtures mirroring the exact JSON shapes Rust serializes over IPC.
//
// The Rust DTOs use `#[serde(rename_all = "camelCase")]` (see
// src-tauri/src/state.rs), so these fixtures deliberately use camelCase
// keys — including the fields that used to be snake_case and caused the
// infinite "正在检查系统钥匙串…" startup freeze. The Rust contract tests in
// src-tauri/tests/state.rs assert the same shapes on the Rust side; change
// both together.

import type { AppStatusDto, BootStatusDto } from "../../types/models";

export function bootStatusFixture(overrides?: Partial<BootStatusDto>): BootStatusDto {
  return {
    version: "0.2.0",
    configured: false,
    configLoaded: true,
    configError: null,
    credentials: { phase: "missing", available: false, error: null },
    ...overrides,
  };
}

export function appStatusFixture(overrides?: Partial<AppStatusDto>): AppStatusDto {
  return {
    version: "0.2.0",
    configured: false,
    configLoaded: true,
    configError: null,
    credentials: { phase: "missing", available: false, error: null },
    trayAvailable: true,
    trayError: null,
    monitor: {
      enabled: false,
      intervalSecs: 300,
      loading: false,
      lastFetchMs: null,
      error: null,
      notificationError: null,
    },
    float: { open: false, mode: "full", topDocked: false },
    update: {
      status: "idle",
      available: null,
      downloadedVersion: null,
      progress: null,
      error: null,
      lastCheckedMs: null,
      bannerDismissed: false,
    },
    ...overrides,
  };
}
