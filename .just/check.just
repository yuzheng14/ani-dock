set working-directory := '..'

# 检查 Rust 代码
mod rust 'check-rust.just'

# 检查前端代码
mod frontend 'check-frontend.just'

# 检查所有 Rust 和 TypeScript 代码，包含格式化、lint 和单测
[default]
[private]
default: typos rust::default _test-rust frontend::default toml rust::format

[private]
_test-rust:
    cargo nextest run --workspace

# 检查拼写
typos:
    typos

# 检查 TOML 格式
toml:
    tombi fmt --check
