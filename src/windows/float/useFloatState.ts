// Floating-window state hook: the single frontend source of truth for the
// presentation mode.
//
// The backend emits `float://state` on every mode/dock/undock change; the
// frontend renders ONLY from `presentationMode` in that snapshot and keeps no
// local mode copy that could diverge (the "stale local mode" regression).
//
// A slow 10s reconciliation poll remains as a fallback only: native drags
// normally emit the event, but if one is ever missed the snapshot is fetched
// and applied — and it never causes a render when nothing changed.

import { useEffect, useState } from "react";
import { api } from "../../services/tauri";
import { EVENT, type FloatStateDto } from "../../types/models";
import { useTauriEvent } from "../../hooks/useTauriEvents";
import { snapshotEqual } from "./floatLayout";

/** Fallback reconciliation interval in ms (10 s, not 1 s). */
const FALLBACK_POLL_MS = 10_000;

export function useFloatState(): FloatStateDto | null {
  const [floatState, setFloatState] = useState<FloatStateDto | null>(null);

  // Event-driven: every backend mutation emits the canonical snapshot.
  useTauriEvent<FloatStateDto>(EVENT.FLOAT_STATE, setFloatState);

  // Initial snapshot; the component renders a lightweight boot shell until
  // this resolves instead of guessing a mode.
  useEffect(() => {
    let cancelled = false;
    void api
      .getFloatState()
      .then((state) => {
        if (!cancelled) setFloatState(state);
      })
      .catch((error) => {
        console.error("[float] get_float_state failed", error);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Fallback reconciliation; no-op when the snapshot is unchanged.
  useEffect(() => {
    const timer = window.setInterval(() => {
      void api.getFloatState().then((state) => {
        setFloatState((current) => (snapshotEqual(current, state) ? current : state));
      });
    }, FALLBACK_POLL_MS);
    return () => window.clearInterval(timer);
  }, []);

  return floatState;
}
