# Codex + GrowthBook bridge

This package connects the public Codex CLI feature settings to a GrowthBook project
using the official GrowthBook Rust SDK. It is a sidecar launcher; it does not patch
the upstream Codex source or alter the Codex user configuration file.

## What it does

1. Fetches a GrowthBook feature payload with growthbook-rust.
2. Evaluates a strict allowlist of public, stable Codex feature keys.
3. Starts the installed Codex CLI with the corresponding --enable / --disable
   options.
4. Leaves Codex's existing settings unchanged when a GrowthBook key is absent.
5. Stops before launching Codex if the configured GrowthBook fetch or evaluation
   fails.

The launcher never invokes a shell to start Codex. GrowthBook values cannot become
arbitrary command-line arguments: they can only select the fixed keys in
src/lib.rs. GrowthBook credentials and targeting attributes are removed from the
child process environment.

## Feature map

Create these boolean features in your GrowthBook project. Use defaultValue to
set a decision for the corresponding Codex setting.

| GrowthBook key | Codex feature | Codex default |
| --- | --- | --- |
| codex.shell_tool | shell_tool | enabled |
| codex.unified_exec | unified_exec | enabled |
| codex.shell_snapshot | shell_snapshot | enabled |
| codex.view_image | view_image | enabled |
| codex.sleep_tool | sleep_tool | enabled |
| codex.hooks | hooks | enabled |
| codex.multi_agent | multi_agent | enabled |

Only these public stable settings are mapped. Internal OpenAI rollout keys, account
eligibility, authentication, managed policy, sandbox policy, and experimental or
under-development features are not mapped. In particular, this bridge does not
provide OpenAI's internal GrowthBook project, SDK key, or private targeting rules.

Codex still applies its own configuration validation and managed-feature
requirements. A GrowthBook decision is a user-config preference, not an
authorization override.

## Build

The parent repository pins Codex as a Git submodule. Initialize it and build Codex:

~~~sh
git submodule update --init --recursive
cd vendor/codex/codex-rs
cargo build --release -p codex-cli
~~~

Build the bridge in a separate terminal from the repository root:

~~~sh
cd growthbook
cargo test
cargo build --release
~~~

The bridge executable is growthbook/target/release/codex-growthbook.

## Configure

Set the GrowthBook SDK client key in the environment. Do not put it in Git or
paste it into chat.

~~~sh
export GB_SDK_KEY='YOUR_GROWTHBOOK_SDK_CLIENT_KEY'
export GB_URL='https://cdn.growthbook.io'
export CODEX_BIN="$PWD/vendor/codex/codex-rs/target/release/codex"
~~~

GB_URL is optional; the shown value is the default. Set CODEX_BIN only when Codex
is not available on PATH.

Optional targeting attributes can be supplied explicitly as a JSON object:

~~~sh
export GB_ATTRIBUTES_JSON='{"environment":"development","team":"engineering"}'
~~~

If the GrowthBook project uses encrypted features, set GB_DECRYPTION_KEY in the
environment. The launcher does not generate user attributes or infer a user ID.

## Use

From the growthbook directory:

~~~sh
./target/release/codex-growthbook list
./target/release/codex-growthbook run -- --help
./target/release/codex-growthbook run -- -p "Summarize this repository"
~~~

If Codex is installed elsewhere, set CODEX_BIN to its executable path.

## Failure behavior and limits

- Missing GB_SDK_KEY, network errors, invalid feature payloads, or non-boolean
  mapped values stop the launcher; Codex is not started.
- A missing GrowthBook feature is reported as unconfigured and does not disable a
  Codex feature.
- The bridge does not persist remote values to the Codex config file.
- The bridge fetches once at launch; it does not run a background refresh loop.
- GrowthBook SDK keys are client keys, not private service credentials.
- The upstream Codex submodule and its Apache-2.0 license/notices are untouched.

## SDK

Uses growthbook-rust 0.4.0, the GrowthBook Rust SDK (MIT):
https://github.com/growthbook/growthbook-rust.
