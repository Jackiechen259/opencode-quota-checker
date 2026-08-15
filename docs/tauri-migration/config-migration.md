# Config migration (Iced → Tauri)

## Path and format

Both clients use `directories::BaseDirs` and write:

```text
<config-dir>/opencode-quota-checker/config.json
```

so an existing Iced installation's configuration is read in place by the
Tauri build — no copy or conversion step. On Windows this is
`%APPDATA%\opencode-quota-checker\config.json`.

The Tauri `AppConfig` struct is field-identical to the archived Iced model
(schema 2): workspace id, monitor enabled/interval, thresholds, close
behavior, float open/mode/position, update checks + auto-download. Loading
uses `#[serde(default)]`, so:

- schema 2 files load unchanged (test: `schema_2_config_keeps_every_existing_field`);
- schema 1 files (pre-updater era) load with update toggles defaulting to
  enabled (test: `schema_1_config_migrates_with_new_update_defaults`);
- unknown/extra fields are ignored; a corrupt file surfaces a warning in the
  settings page and the app falls back to defaults without destroying the
  file (no silent overwrite).

Writes are atomic (same-directory temp file + rename + fsync), identical to
the Iced store.

## Credentials

The auth cookie never enters the config file in either client. Both use
`opencode-core::OpenCodeAuthStore` with the identical keyring namespace:

```text
service = "opencode-quota-checker"
account = "opencode-auth"
```

An Iced-saved cookie is therefore readable by the Tauri build without user
action. Verified end-to-end on Windows Credential Manager
(`LegacyGeneric:target=opencode-auth.opencode-quota-checker`).

## Future schema bumps

If a later version adds fields, bump `SCHEMA_VERSION` and add an explicit
migration in `ConfigStore::load` (never let an old config silently lose
data); keep `#[serde(default)]` for forward compatibility.
