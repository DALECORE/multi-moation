# Offline Codex + GrowthBook export bridge

This is a local-file integration. It reads a previously exported GrowthBook JSON
file and never connects to GrowthBook Cloud or any other network service.

## Behavior

- Reads a flat JSON export (top-level object) from disk.
- Preserves native JSON values.
- Maps only the explicit `codex.*` keys listed below.
- Ignores unrelated keys, including Claude Code `tengu_*` and `cai_*` flags.
- Accepts either a direct boolean value or a GrowthBook feature envelope with a
  boolean `defaultValue`.
- Rejects non-boolean values for mapped Codex keys.
- Does not change Codex's user config file or bypass Codex policy/authorization.

## Feature map

| Export key | Codex CLI feature |
| --- | --- |
| `codex.shell_tool` | `shell_tool` |
| `codex.unified_exec` | `unified_exec` |
| `codex.shell_snapshot` | `shell_snapshot` |
| `codex.view_image` | `view_image` |
| `codex.sleep_tool` | `sleep_tool` |
| `codex.hooks` | `hooks` |
| `codex.multi_agent` | `multi_agent` |

**Important:** the previously discussed export contains Claude Code flag names
such as `tengu_*` and `cai_*`. Those are not Codex flags and are not silently
translated into Codex settings. Unless the export contains one of the explicit
`codex.*` keys above, the bridge reports the Codex settings as unconfigured and
leaves Codex defaults/configuration alone.

## Build

From the repository root:

~~~sh
cd growthbook
cargo fmt -- --check
cargo test
cargo build --release
~~~

## Use

Point the bridge at the exported JSON file:

~~~sh
export GROWTHBOOK_FEATURES_FILE="/sdcard/Download/growthbook-export/all-features.json"
./target/release/codex-growthbook list
./target/release/codex-growthbook run -- --help
~~~

The default path is `growthbook/all-features.json` relative to the current
working directory. Override it with `GROWTHBOOK_FEATURES_FILE` when the export
is stored elsewhere.

Set `CODEX_BIN` if the Codex executable is not on `PATH`.

## Offline guarantee

The package has no GrowthBook SDK dependency, no HTTP client dependency, no SDK
key, no API URL, no async runtime, and no network-fetch code. Reading the local
JSON file is the only feature-data input.
