setup-db:
  cargo sqlx db setup --source crates/ani-dock-db/migrations

reset-db:
  cargo sqlx db reset --source crates/ani-dock-db/migrations

run-server:
  cargo run -p ani-dock-server

check: prettier-check typecheck lint
  cargo check --all-targets --workspace
  typos
  tombi fmt --check
  cargo fmt --check

test-e2e:
  RUST_LOG=ani_dock_core=debug cargo nextest run -p e2e --profile e2e --no-capture

renew-git-hook:
  # cp -p .git-hooks/* .git/hooks/
  git config --local core.hooksPath .git-hooks

@setup: renew-git-hook
  typos --version || echo "请使用 brew install typos-cli 安装 typos 用于拼写检查"
  tombi --version || echo "请使用 brew install tombi 安装 tombi 用于 toml 文件格式化"
  sqlx --version && just setup-db || echo "请使用 brew install sqlx-cli 安装 sqlx 用于数据库相关操作"

prettier-check:
  pnpm -F frontend exec prettier --check .

typecheck:
  pnpm -F frontend exec tsc -b --noEmit

lint:
  pnpm -F frontend exec oxlint --deny-warnings

dev-fe:
  pnpm -F frontend dev

build-fe:
  pnpm -F frontend build

