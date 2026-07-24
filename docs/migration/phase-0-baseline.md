# Phase 0 baseline

Captured on 2026-07-24 from commit `509516a`.

The recoverable source baseline is the annotated Git tag
`pre-iced-refactor-20260724`.

## Existing feature inventory

- System-keyring storage for one Access Key / Secret Key pair.
- Signed `GetAFPUsage` requests with a 15-second HTTP timeout.
- Five-hour, weekly, and monthly usage windows.
- Used, quota, remaining, percentage, subscription cycle, and reset data.
- Manual refresh and raw JSON retrieval.
- Configurable monitor interval and per-window alert thresholds.
- Desktop notifications with per-subscription-cycle deduplication.
- Always-on-top floating usage window.
- Full and compact floating presentation modes, including top docking.
- Settings persistence through the Tauri store.
- Windows, Linux, macOS Intel, and macOS Apple Silicon release jobs.

## Baseline artifacts and measurements

The checked-in `dock-8.png` is the available floating-window visual reference.
The existing generated web bundle in `dist/` totals 121,767 bytes:

| Artifact | Bytes |
| --- | ---: |
| `dist/index.html` | 672 |
| `dist/assets/index-DhJE-HP4.js` | 92,919 |
| `dist/assets/index-DLBJX9RS.css` | 28,176 |

No installer or release executable is present in this checkout, so installed
size, steady-state memory, and cold-start timing cannot be measured
reproducibly here. They remain comparison items for packaged smoke testing.

## Legacy behavior acceptance checklist

- [ ] First launch shows credential entry when the keyring entry is absent.
- [ ] Saving non-empty credentials enables usage retrieval.
- [ ] Deleting credentials clears usage and stops monitoring.
- [ ] The three displayed quota windows match the API response.
- [ ] Manual refresh preserves the last report when a later request fails.
- [ ] Raw JSON can be viewed for debugging.
- [ ] Monitoring respects its configured interval.
- [ ] A threshold alert is sent once per subscription cycle.
- [ ] Floating-window mode and position survive restart.
- [ ] Closing the main window does not unintentionally leave an unrecoverable
      background process.
- [ ] The explicit exit action stops the process.

## Baseline verification

The original Rust unit tests cover response parsing, percentage clamping,
truncation, canonical query generation, signing-key derivation, and signing
determinism. Cross-platform installed-package behavior is intentionally not
claimed by this document; it must be verified by the release gates in
`checklist.md`.

