// Floating-window snapshot: the single frontend source of truth for the
// presentation mode.
//
// The backend emits `float://state` on every mode/dock/undock change; the
// frontend renders ONLY from `presentationMode` in that snapshot and keeps no
// local mode copy that could diverge.
//
// Two races are guarded here:
//   * a late initial/poll response must not overwrite a newer event;
//   * unchanged snapshots are applied as the same object, so the 10s fallback
//     poll cannot cause a re-render when nothing changed.

import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../services/tauri";
import { EVENT, type FloatStateDto } from "../../types/models";
import { useTauriEvent } from "../../hooks/useTauriEvents";
import { snapshotEqual } from "./floatLayout";

/** Fallback reconciliation interval in ms (10 s, not 1 s). */
const FALLBACK_POLL_MS = 10_000;

export function useFloatSnapshot(): FloatStateDto | null {
  const [snapshot, setSnapshot] = useState<FloatStateDto | null>(null);
  /** Increments on every event; a fetch started before it is stale. */
  const eventSequence = useRef(0);

  useTauriEvent<FloatStateDto>(EVENT.FLOAT_STATE, (payload) => {
    eventSequence.current += 1;
    setSnapshot(payload);
  });

  const applySnapshot = useCallback((next: FloatStateDto, sequence: number) => {
    if (eventSequence.current !== sequence) return;
    setSnapshot((current) => (snapshotEqual(current, next) ? current : next));
  }, []);

  // Initial snapshot; the component renders a lightweight boot shell until
  // this resolves instead of guessing a mode.
  useEffect(() => {
    let cancelled = false;
    const sequence = eventSequence.current;
    api
      .getFloatState()
      .then((next) => {
        if (!cancelled) applySnapshot(next, sequence);
      })
      .catch((error) => {
        console.error("[float] get_float_state failed", error);
      });
    return () => {
      cancelled = true;
    };
  }, [applySnapshot]);

  // Fallback reconciliation; no-op when the snapshot is unchanged.
  useEffect(() => {
    const timer = window.setInterval(() => {
      const sequence = eventSequence.current;
      api
        .getFloatState()
        .then((next) => applySnapshot(next, sequence))
        .catch((error) => {
          console.error("[float] get_float_state failed", error);
        });
    }, FALLBACK_POLL_MS);
    return () => window.clearInterval(timer);
  }, [applySnapshot]);

  return snapshot;
}
