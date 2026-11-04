# Multi-Moation

This repository pins the official OpenAI Codex CLI source as a Git submodule at vendor/codex.

## Clone with Codex source

~~~sh
git clone --recurse-submodules https://github.com/DALECORE/multi-moation.git
~~~

If the repository is already cloned:

~~~sh
git submodule update --init --recursive
~~~

## Upstream source

- Repository: https://github.com/openai/codex
- Pinned upstream commit: f39e45e4a68db24c649a9b6b12d2e922cba616d1
- License: Apache-2.0

The upstream source, LICENSE, and NOTICE remain in the submodule checkout. This repository does not claim ownership of OpenAI's code. To update Codex, review an upstream commit and deliberately move the submodule pointer.

## GrowthBook integration

The growthbook directory contains a separate Rust launcher using the official GrowthBook Rust SDK. It fetches boolean GrowthBook features and maps only an allowlist of public, stable Codex settings to the Codex CLI's supported --enable / --disable arguments.

See growthbook/README.md for the feature map, configuration, build, and run instructions. The integration does not patch vendor/codex, modify the Codex config file, or access OpenAI's private/internal GrowthBook project or flags.
