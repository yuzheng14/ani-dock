setup-db:
  cargo sqlx db setup --source crates/ani-dock-db/migrations

reset-db:
  cargo sqlx db reset --source crates/ani-dock-db/migrations

run-server:
  cargo run -p ani-dock-server

check:
  cargo check --all-targets --workspace

test-e2e:
  RUST_LOG=ani_dock_core=debug cargo nextest run -p e2e --profile e2e --no-capture
