set working-directory := '..'

[private]
default:
    @just --justfile "{{ justfile() }}" --list test

# 使用 nextest 运行 Rust 侧默认测试
rust +args="":
    cargo nextest run --workspace {{ args }}

# 运行需要真实 Cookie 的 Rust E2E 测试
e2e:
    RUST_LOG=info,ani_dock_core=debug cargo nextest run -p e2e --profile e2e --no-capture
