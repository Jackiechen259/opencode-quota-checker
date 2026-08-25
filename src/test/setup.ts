import "@testing-library/jest-dom/vitest";

// jsdom < 26 has no PointerEvent; the window chrome handlers use
// `onPointerDown` so without a polyfill `fireEvent.pointerDown` creates a
// generic Event that React never delivers as a PointerEvent.
if (typeof window !== "undefined" && typeof (window as unknown as Record<string, unknown>).PointerEvent === "undefined") {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (window as any).PointerEvent = window.MouseEvent;
}
if (typeof globalThis !== "undefined" && typeof (globalThis as unknown as Record<string, unknown>).PointerEvent === "undefined") {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (globalThis as any).PointerEvent = (window as unknown as Record<string, any>).MouseEvent;
}
