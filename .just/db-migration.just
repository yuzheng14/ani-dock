set working-directory := '..'
set default-list

# 新增 migration 文件
add migration-name:
    cargo sqlx migrate add --source crates/ani-dock-db/migrations {{ migration-name }}

# 应用所有待执行的 migrations
run:
    cargo sqlx migrate run --source crates/ani-dock-db/migrations
