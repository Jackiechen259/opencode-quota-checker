// Application-level hooks binding the Rust backend to React state.

import { useCallback, useEffect, useState } from "react";
import { api } from "../services/tauri";
import {
  EVENT,
  type AppConfig,
  type AppError,
  type AppStatusDto,
  type BootStatusDto,
  type GoOverviewDto,
  type TokenUsageDto,
  type UsageReport,
  type UpdateStateDto,
} from "../types/models";
import { useTauriEvent } from "./useTauriEvents";

/**
 * Startup state machine of the main window.
 *
 * A failed or hanging `get_boot_status` must surface as `error` — or hit the
 * startup watchdog — and must never be misinterpreted as "still loading".
 * `status === null` no longer exists as a state.
 */
export type BootState =
  | { phase: "loading" }
  | { phase: "ready"; status: BootStatusDto }
  | { phase: "error"; error: unknown };

/**
 * Boot-critical application status plus the full runtime snapshot.
 *
 * Boot gating reads only `boot` (from `get_boot_status`): a wedged updater,
 * monitor, float or tray subsystem can never prevent the main window from
 * starting. `status` carries the full `AppStatusDto` for display-only
 * consumers (tray badge, settings badges) and tolerates its own failure.
 */
export function useAppStatus() {
  const [boot, setBoot] = useState<BootState>({ phase: "loading" });
  const [status, setStatus] = useState<AppStatusDto | null>(null);

  const reload = useCallback(() => {
    api
      .getBootStatus()
      .then((bootStatus) => setBoot({ phase: "ready", status: bootStatus }))
      .catch((error) => {
        console.error("[startup] get_boot_status failed", error);
        setBoot({ phase: "error", error });
      });
    // Display-only full snapshot; its failure must not fail the boot.
    api
      .getAppStatus()
      .then(setStatus)
      .catch((error) => {
        console.error("[startup] get_app_status failed", error);
        setStatus(null);
      });
  }, []);

  /** Re-runs the boot status fetch from the startup error screen. */
  const retry = useCallback(() => {
    setBoot({ phase: "loading" });
    reload();
  }, [reload]);

  useEffect(() => {
    reload();
  }, [reload]);

  useTauriEvent<AppStatusDto>(EVENT.APP_STATUS, (payload) => {
    setStatus(payload);
    // The event payload is the full AppStatusDto — a superset of the boot
    // DTO — so a ready boot state can safely adopt it.
    setBoot((current) =>
      current.phase === "ready" ? { phase: "ready", status: payload } : current,
    );
  });

  return { boot, status, reload, retry };
}

/** Latest usage report; updated by quota events. */
export function useQuota() {
  const [report, setReport] = useState<UsageReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [lastSuccessMs, setLastSuccessMs] = useState<number | null>(null);

  const reload = useCallback(() => {
    api
      .getUsage()
      .then((usage) => {
        setReport(usage.report);
        setLoading(usage.loading);
        setError(usage.error);
        setLastSuccessMs(usage.lastSuccessMs);
      })
      .catch((caught) => {
        console.error("[usage] get_usage failed", caught);
        setError(caught as AppError);
      });
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  useTauriEvent<UsageReport>(EVENT.QUOTA_UPDATED, (payload) => {
    setReport(payload);
    setError(null);
    setLastSuccessMs(Date.now());
  });
  useTauriEvent<AppError>(EVENT.QUOTA_ERROR, setError);
  useTauriEvent<boolean>(EVENT.MONITOR_STATUS, () => reload());

  return { report, loading, error, lastSuccessMs, reload };
}

/** Latest workspace token usage; updated by `token://state` events. */
export function useTokenUsage() {
  const [token, setToken] = useState<TokenUsageDto>({
    report: null,
    loading: false,
    error: null,
    lastSuccessMs: null,
  });

  const reload = useCallback(() => {
    api
      .getTokenUsage()
      .then(setToken)
      .catch((caught) => {
        console.error("[token-usage] get_token_usage failed", caught);
        setToken((current) => ({
          ...current,
          loading: false,
          error: caught as AppError,
        }));
      });
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  useTauriEvent<TokenUsageDto>(EVENT.TOKEN_USAGE_STATE, setToken);
  // Safety net for the startup race: a very fast token response can land
  // before the listener above is registered. The monitor emits this event at
  // the end of each quota cycle, so the snapshot is re-read then.
  useTauriEvent<unknown>(EVENT.MONITOR_STATUS, () => reload());

  return token;
}

const EMPTY_GO_OVERVIEW: GoOverviewDto = {
  status: null,
  activity: null,
  billing: null,
  statusError: null,
  activityError: null,
  billingError: null,
  loading: false,
  lastSuccessMs: null,
};

/**
 * On-demand Go overview for the Go usage page.
 *
 * The backend starts the fetch when the page asks for it; this hook listens for
 * `go://overview`, retries a short poll while loading (event-race safety), and
 * refreshes every five minutes while the page is mounted.
 */
export function useGoOverview() {
  const [data, setData] = useState<GoOverviewDto>(EMPTY_GO_OVERVIEW);

  const reload = useCallback(() => {
    api
      .getGoOverview()
      .then(setData)
      .catch((caught) => {
        console.error("[go-overview] get_go_overview failed", caught);
        setData((current) => ({
          ...current,
          loading: false,
          statusError: caught as AppError,
        }));
      });
  }, []);

  const refresh = useCallback(() => {
    api
      .refreshGoOverview()
      .then(setData)
      .catch((caught) => {
        console.error("[go-overview] refresh_go_overview failed", caught);
        setData((current) => ({
          ...current,
          loading: false,
          statusError: caught as AppError,
        }));
      });
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  useTauriEvent<GoOverviewDto>(EVENT.GO_OVERVIEW_STATE, setData);

  // A fast response can beat the listener registration on first mount.
  useEffect(() => {
    if (!data.loading) return;
    const timer = window.setTimeout(reload, 2_000);
    return () => window.clearTimeout(timer);
  }, [data.loading, reload]);

  // Keep the open page current without adding a background monitor request.
  useEffect(() => {
    const timer = window.setInterval(refresh, 5 * 60 * 1_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  return { data, refresh, reload };
}

/** Updater snapshot; refreshed by `update://state` events. */
export function useUpdater() {
  const [update, setUpdate] = useState<UpdateStateDto | null>(null);

  const reload = useCallback(() => {
    api
      .getUpdateState()
      .then(setUpdate)
      .catch((error) => {
        console.error("[updater] get_update_state failed", error);
        setUpdate(null);
      });
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  useTauriEvent<UpdateStateDto>(EVENT.UPDATE_STATE, setUpdate);
  return { update, reload };
}

/** Persisted configuration; refreshed with the canonical saved value. */
export function useConfig() {
  const [config, setConfig] = useState<AppConfig | null>(null);

  const reload = useCallback(() => {
    api
      .getConfig()
      .then(setConfig)
      .catch((error) => {
        console.error("[config] get_config failed", error);
        setConfig(null);
      });
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  return { config, setConfig, reload };
}
