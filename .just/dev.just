set working-directory := '..'

[private]
default:
    @just --justfile "{{ justfile() }}" --list dev

# 运行后端服务
server:
    cargo run -p ani-dock-server

# 启动前端开发服务器
frontend:
    pnpm -F @ani-dock/frontend dev
