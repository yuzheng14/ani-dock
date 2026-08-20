# Repository Instructions

## Rust tests

- Use `just nextest` or `cargo nextest run --workspace` for the default Rust test suite.
- Do not run an unfiltered `cargo test`. The `ts-rs` tests named `export_bindings_*` write to `packages/shared-type/types` and can overwrite the tracked, Prettier-formatted TypeScript bindings.
- If ordinary Cargo tests are specifically required, exclude the binding exporters with `cargo test <args> -- --skip export_bindings`.
- Generate TypeScript bindings only through `just gen-type`; this intentionally runs the exporter tests and formats their output.
- Do not edit files under `packages/shared-type/types` manually.

## HTTP JSON responses

- Whenever parsing an HTTP response body as JSON, use the project's `JsonResponseExt::json_or_log` wrapper instead of the HTTP client's native `.json()` method.
