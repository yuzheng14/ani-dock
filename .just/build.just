set working-directory := '..'

[private]
default:
    @just --justfile "{{ justfile() }}" --list build

# 构建前端
frontend:
    pnpm -F @ani-dock/frontend build
