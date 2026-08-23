set working-directory := '..'
set default-list

e2e_rust_log := env_var_or_default("RUST_LOG", "info,ani_dock_core=debug")

# 使用 nextest 运行 Rust 侧默认测试
rust +args="":
    cargo nextest run --workspace {{ args }}

# 运行前端单元测试
frontend:
    pnpm -F @ani-dock/frontend test

# 运行需要真实 Cookie 的 Rust E2E 测试
e2e:
    RUST_LOG='{{ e2e_rust_log }}' cargo nextest run -p e2e --profile e2e --no-capture
