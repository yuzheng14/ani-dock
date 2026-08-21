set working-directory := '..'

[private]
default:
    @just --justfile "{{ justfile() }}" --list db

# 初始化数据库
setup:
    cargo sqlx db setup --source crates/ani-dock-db/migrations

# 重置数据库
reset:
    cargo sqlx db reset --source crates/ani-dock-db/migrations
