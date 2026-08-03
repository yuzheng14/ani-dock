# 检查所有 just 配方
default:
  @just --list

# 初始化数据库
setup-db:
  cargo sqlx db setup --source crates/ani-dock-db/migrations

# 重置数据库
reset-db:
  cargo sqlx db reset --source crates/ani-dock-db/migrations

# 运行后端服务
run-server:
  cargo run -p ani-dock-server

# 执行 nextest 以运行 rust 侧单测
nextest +args="":
  cargo nextest run --workspace {{args}}

# 从 rust 类型生成 ts 类型
gen-type:
  cargo nextest run --ignore-default-filter export_bindings
  pnpm -F @ani-dock/frontend exec prettier --write ../shared-type/types

cargo-check:
  cargo check --all-targets --workspace

typos:
  typos

# 检查所有的 rust ts 检查，包含格式化 lint 单测
check: typos cargo-check nextest prettier-check typecheck lint
  tombi fmt --check
  cargo fmt --check

# 运行 rust 的 e2e 测试，需要真实的 cookie
test-e2e:
  RUST_LOG=ani_dock_core=debug cargo nextest run -p e2e --profile e2e --no-capture

# 更新 git 钩子
renew-git-hook:
  git config --local core.hooksPath .git-hooks

# 初始化开发环境及检查，刚 clone 后必须执行通过此指令
@setup: renew-git-hook
  typos --version || echo "请使用 brew install typos-cli 安装 typos 用于拼写检查"
  tombi --version || echo "请使用 brew install tombi 安装 tombi 用于 toml 文件格式化"
  sqlx --version && just setup-db || echo "请使用 brew install sqlx-cli 安装 sqlx 用于数据库相关操作"

# 运行 prettier 检查前端代码格式化
prettier-check:
  pnpm -F @ani-dock/frontend exec prettier --check .

# 运行前端代码的类型检查
typecheck:
  pnpm -F @ani-dock/frontend exec tsc -b --noEmit

# 使用 oxlint 检查前端代码
lint:
  pnpm -F @ani-dock/frontend exec oxlint --deny-warnings

# 启动前端开发服务器
dev-fe:
  pnpm -F @ani-dock/frontend dev

# 构建前端
build-fe:
  pnpm -F @ani-dock/frontend build

add-shadcn-component component-name:
  pnpm -F @ani-dock/frontend exec shadcn add {{component-name}}
