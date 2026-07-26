setup-db:
  cargo sqlx db setup --source crates/ani-dock-db/migrations

reset-db:
  cargo sqlx db reset --source crates/ani-dock-db/migrations
