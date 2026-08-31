# Repository Instructions

## Rust tests

- Use `just test rust` or `cargo nextest run --workspace` for the default Rust test suite.
- Do not run an unfiltered `cargo test`. The `ts-rs` tests named `export_bindings_*` write to `packages/shared-type/types` and can overwrite the tracked, Prettier-formatted TypeScript bindings.
- If ordinary Cargo tests are specifically required, exclude the binding exporters with `cargo test <args> -- --skip export_bindings`.
- Generate TypeScript bindings only through `just generate types`; this intentionally runs the exporter tests and formats their output.
- Do not edit files under `packages/shared-type/types` manually.

## HTTP JSON responses

- Whenever parsing an HTTP response body as JSON, use the project's `JsonResponseExt::json_or_log` wrapper instead of the HTTP client's native `.json()` method.

## GitHub issues and pull requests

- Write all issues and pull requests entirely in English.
- Add at least one applicable priority label to every issue.

## Production code changes

- Do not modify production code unless the user has explicitly approved direct production-code edits.
- Default to an "explain, then the user edits" workflow: describe the issue and the exact recommended change, then let the user apply it.
- Requests to review, diagnose, or add tests do not authorize production-code fixes or refactors.
- If a test would require a production-code seam or refactor, stop and obtain the user's explicit approval before making that change.
