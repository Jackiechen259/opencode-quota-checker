# Iced native migration checklist

This checklist tracks the implementation described in
`volc_status_iced_refactor_plan.md`. Platform smoke tests that cannot be run on
the current host remain explicit release gates instead of being recorded as
successful.

## Phase status

- [x] Phase 0: baseline captured
- [x] Phase 1: Iced daemon and tray lifecycle spike
- [x] Phase 2: Cargo workspace and core extraction
- [x] Phase 3: Iced application skeleton
- [x] Phase 4: credentials and API
- [x] Phase 5: main dashboard
- [ ] Phase 6: settings, monitoring, and notifications
- [ ] Phase 7: floating window
- [ ] Phase 8: debug view and UX polish
- [ ] Phase 9: packaging and CI/CD
- [ ] Phase 10: remove the legacy web stack

## Cross-platform release gates

- [ ] Windows installed-package smoke test
- [ ] macOS Intel installed-package smoke test
- [ ] macOS Apple Silicon installed-package smoke test
- [ ] Ubuntu installed-package smoke test
- [ ] Windows tray lifecycle manual test
- [ ] macOS tray lifecycle manual test
- [ ] Ubuntu tray lifecycle manual test
- [ ] Windows notification and keyring manual test
- [ ] macOS notification and keyring manual test
- [ ] Ubuntu notification and keyring manual test
