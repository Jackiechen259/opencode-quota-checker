// Application-level hooks binding the Rust backend to React state.

import { useCallback, useEffect, useState } from "react";
import { api } from "../services/tauri";
import {
  EVENT,
  type AppConfig,
  type AppError,
  type AppStatusDto,
  type UsageReport,
  type UpdateStateDto,
} from "../types/models";
import { useTauriEvent } from "./useTauriEvents";

/** Full application status; refreshed on `app://status` events. */
export function useAppStatus() {
  const [status, setStatus] = useState<AppStatusDto | null>(null);

  const reload = useCallback(() => {
    void api.getAppStatus().then(setStatus).catch(() => {});
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  useTauriEvent<AppStatusDto>(EVENT.APP_STATUS, setStatus);
  return { status, reload };
}

/** Latest usage report; updated by quota events. */
export function useQuota() {
  const [report, setReport] = useState<UsageReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [lastSuccessMs, setLastSuccessMs] = useState<number | null>(null);

  const reload = useCallback(() => {
    void api
      .getUsage()
      .then((usage) => {
        setReport(usage.report);
        setLoading(usage.loading);
        setError(usage.error);
        setLastSuccessMs(usage.last_success_ms);
      })
      .catch(() => {});
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

/** Updater snapshot; refreshed by `update://state` events. */
export function useUpdater() {
  const [update, setUpdate] = useState<UpdateStateDto | null>(null);

  const reload = useCallback(() => {
    void api.getUpdateState().then(setUpdate).catch(() => {});
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
    void api.getConfig().then(setConfig).catch(() => {});
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  return { config, setConfig, reload };
}
