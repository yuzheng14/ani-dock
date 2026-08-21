set working-directory := '..'

[private]
default:
    @just --justfile "{{ justfile() }}" --list ui

# 添加 shadcn/ui 组件
add component-name:
    pnpm -F @ani-dock/frontend exec shadcn add {{ component-name }}
