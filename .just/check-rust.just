set working-directory := '..'

# 使用 cargo check 检查 Rust 代码
[private]
default:
    cargo check --all-targets --workspace

# 检查 Rust 代码格式
format:
    cargo fmt --check
