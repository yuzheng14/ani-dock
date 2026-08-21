set working-directory := '..'
set default-list

# 从 Rust 类型生成 TypeScript 类型
types:
    cargo nextest run --ignore-default-filter export_bindings
    pnpm -F @ani-dock/frontend exec prettier --write ../shared-type/types
